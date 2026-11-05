//! Indexing a file's fragments as its bytes arrive.

use std::collections::HashMap;

use crate::boxes::{Found, Header, find, lenient_kind};
use crate::claims::Claims;
use crate::fragment::{self, Context, Fault};
use crate::movie::{Movie, Track};
use crate::table::{Description, Fragment, SampleTable};
use crate::{Error, SampleTable as Table};

/// Indexes a file's fragments as its bytes arrive, each once it is whole.
///
/// Made from the file's [`Movie`], it takes the file so far with [`advance`](Self::advance) as
/// often as the caller likes, and [`finish`](Self::finish) once the file is whole. It never reads
/// a fragment twice, nor the movie box: the caller may make them clear meanwhile
/// ([`make_clear`](crate::make_clear)). Its [`samples`](Self::samples) grow by whole fragments.
#[derive(Debug)]
pub struct Indexer {
    tracks: Vec<Track>,
    /// Where the first top-level box not yet whole, or whose samples have not all arrived, starts.
    position: usize,
    /// Every box starting before this has been seen, and registered if it is an `mdat`.
    scanned_to: usize,
    table: SampleTable,
    /// The `mdat` boxes seen: their payload's start and end, in file order.
    mdats: Vec<(usize, usize)>,
    descriptions: HashMap<Description, u32>,
    /// The bytes the samples indexed own, each by one sample.
    claims: Claims,
    /// The `moof` at `position` was read, and waits for the file up to this offset.
    pending_needs: Option<usize>,
    /// Boxes, samples and entries read, for tests that bound the work done.
    steps: usize,
}

impl Indexer {
    /// An indexer for the fragments that follow `movie`'s box.
    pub fn new(movie: &Movie) -> Self {
        Self {
            tracks: movie.tracks.clone(),
            position: movie.end,
            scanned_to: movie.end,
            table: SampleTable::default(),
            mdats: Vec::new(),
            descriptions: HashMap::new(),
            claims: Claims::default(),
            pending_needs: None,
            steps: 0,
        }
    }

    /// Indexes the fragments of `data`, the whole file so far from its start, that are whole:
    /// their `moof` and every byte their samples reference have arrived.
    ///
    /// Returns where the file stops being whole: every byte before it belongs to the movie box,
    /// to a fragment indexed or to another whole top-level box; the first box not yet whole, or
    /// a `moof` whose samples have not all arrived, starts there.
    ///
    /// # Errors
    ///
    /// A malformed fragment: a box with a bad size, a nest deeper than 16, a count larger than
    /// the bytes left, a `tfhd` naming a track or sample entry the movie does not have, a `senc`
    /// whose count or length does not fit its `traf`'s samples (one counting none stands for
    /// samples under a constant IV and without subsamples), subsamples not adding up to
    /// their sample, an 8-byte IV under a CBC scheme, a sample outside every `mdat`, samples
    /// sharing a byte of the file, more zero-size samples than their `moof` has bytes. The
    /// indexer stays where it was: the same error comes back from every later call.
    pub fn advance(&mut self, data: &[u8]) -> Result<usize, Error> {
        self.run(data, false)
    }

    /// The download has ended: `data` is the whole file. Indexes what is left, then gives back
    /// the room the table grew by.
    ///
    /// # Errors
    ///
    /// Those of [`advance`](Self::advance); and a fragment never whole, or a box cut short, is
    /// an error too, the earlier fragments staying indexed.
    pub fn finish(&mut self, data: &[u8]) -> Result<(), Error> {
        self.run(data, true)?;
        self.table.shrink_to_fit();
        Ok(())
    }

    /// Every sample of every fragment indexed so far, in file order.
    pub fn samples(&self) -> &Table {
        &self.table
    }

    pub(crate) fn into_samples(self) -> SampleTable {
        self.table
    }

    pub(crate) fn steps(&self) -> usize {
        self.steps
    }

    /// Indexes from `position` as far as the data allows; `whole` says the data is the whole
    /// file, so that what is not whole is an error and a box of size 0 ends with the data.
    fn run(&mut self, data: &[u8], whole: bool) -> Result<usize, Error> {
        while self.position < data.len() {
            let at = self.position;
            if !whole && self.pending_needs.is_some_and(|needs| data.len() < needs) {
                return Ok(at);
            }
            let header = match find(data, at)? {
                Found::Cut => {
                    if whole {
                        let kind = lenient_kind(data, at + 4).unwrap_or(*b"????");
                        return Err(Error::new(
                            at,
                            kind,
                            format!(
                                "is cut short: the file ends {} bytes into its header",
                                data.len() - at
                            ),
                        ));
                    }
                    return Ok(at);
                }
                Found::Open { kind, body } => {
                    if !whole {
                        return Ok(at);
                    }
                    Header {
                        at,
                        kind,
                        body,
                        end: data.len(),
                    }
                }
                Found::Sized(header) => {
                    if header.end > data.len() {
                        if whole {
                            return Err(Error::new(
                                header.at,
                                header.kind,
                                format!(
                                    "is cut short: it should end at {:#x} but the file ends at {:#x}",
                                    header.end,
                                    data.len()
                                ),
                            ));
                        }
                        return Ok(at);
                    }
                    header
                }
            };
            self.steps += 1;
            match &header.kind {
                b"moof" => match self.fragment(data, header) {
                    Ok(()) => {
                        self.pending_needs = None;
                        if whole && self.table.fragments().len() == 1 {
                            self.reserve_for_rest(data.len(), header);
                        }
                    }
                    Err(Fault::Incomplete { needs }) => {
                        if whole {
                            return Err(Error::new(
                                header.at,
                                header.kind,
                                format!(
                                    "is truncated: its samples need the file up to {needs:#x}, which ends at {:#x}",
                                    data.len()
                                ),
                            ));
                        }
                        self.pending_needs = Some(needs);
                        return Ok(at);
                    }
                    Err(Fault::Malformed(error)) => return Err(error),
                },
                b"mdat" => self.register(header.body, header.end),
                _ => {}
            }
            self.position = header.end;
            self.scanned_to = self.scanned_to.max(header.end);
        }
        Ok(self.position)
    }

    /// Indexes the whole `moof` at `header`, or leaves the table as it was.
    fn fragment(&mut self, data: &[u8], header: Header) -> Result<(), Fault> {
        let mark = self.table.mark();
        let result = self.read_fragment(data, header, mark.samples);
        match result {
            Ok(()) => {
                self.claims.keep();
                self.table.push_fragment(Fragment {
                    start: header.at,
                    first_sample: mark.samples,
                });
            }
            Err(_) => {
                self.claims.give_back();
                self.table.truncate(mark);
            }
        }
        result
    }

    fn read_fragment(&mut self, data: &[u8], header: Header, first: usize) -> Result<(), Fault> {
        let mut cx = Context {
            tracks: &self.tracks,
            table: &mut self.table,
            descriptions: &mut self.descriptions,
            claims: &mut self.claims,
            zero_size_left: header.end - header.at,
            steps: &mut self.steps,
        };
        fragment::moof(data, header, &mut cx)?;
        self.check_in_mdats(data, header, first)
    }

    /// Makes room in the table for the rest of a whole file, at the density of its first
    /// fragment, the `moof` at `moof`: growing the table by doubling, with the copies and the
    /// fresh memory it touches, took a quarter of the time indexing took.
    fn reserve_for_rest(&mut self, file_len: usize, moof: Header) {
        // The first fragment's samples are the table's first.
        let Some(last) = self.table.len().checked_sub(1) else {
            return;
        };
        let start = moof.at.min(self.table.offset(0));
        let end = moof.end.max(
            self.table
                .offset(last)
                .saturating_add(self.table.size(last) as usize),
        );
        let rest = file_len.saturating_sub(end);
        self.table.reserve_for_rest(end - start, rest);
    }

    /// Every sample from `first` on must lie in an `mdat` whose bytes have arrived. A zero-size
    /// sample has no bytes to lie anywhere, but its place is still within the file.
    fn check_in_mdats(&mut self, data: &[u8], moof: Header, first: usize) -> Result<(), Fault> {
        let mut needs: Option<usize> = None;
        // A fragment's samples are mostly in one `mdat`, back to back: searching every `mdat`
        // seen for each of them was a third of the time indexing took.
        let mut last_holding: Option<(usize, usize)> = None;
        for sample in first..self.table.len() {
            let size = self.table.size(sample) as usize;
            let offset = self.table.offset(sample);
            let end = offset.checked_add(size).ok_or_else(|| {
                Error::new(
                    moof.at,
                    moof.kind,
                    "has a sample running past what this machine can address",
                )
            })?;
            if end > data.len() {
                needs = Some(needs.map_or(end, |needs| needs.max(end)));
                continue;
            }
            if size == 0 {
                continue;
            }
            if last_holding.is_some_and(|(start, stop)| start <= offset && end <= stop) {
                continue;
            }
            if self.mdat_holding(offset, end).is_none() {
                self.look_ahead(data, moof.end);
            }
            let holding = self.mdat_holding(offset, end).ok_or_else(|| {
                Error::new(
                    moof.at,
                    moof.kind,
                    format!(
                        "has sample {} at {offset:#x} outside every `mdat`",
                        sample - first
                    ),
                )
            })?;
            last_holding = Some(holding);
        }
        match needs {
            Some(needs) => Err(Fault::Incomplete { needs }),
            None => Ok(()),
        }
    }

    /// The payload's start and end of the `mdat` that holds `offset..end`, if one does.
    fn mdat_holding(&self, offset: usize, end: usize) -> Option<(usize, usize)> {
        let after = self.mdats.partition_point(|(start, _)| *start <= offset);
        let candidate = *self.mdats.get(after.checked_sub(1)?)?;
        (end <= candidate.1).then_some(candidate)
    }

    /// Registers the `mdat` boxes after `from` whose headers have arrived: a fragment's data may
    /// follow its `moof`, which the main walk has not passed yet.
    fn look_ahead(&mut self, data: &[u8], from: usize) {
        let mut at = self.scanned_to.max(from);
        loop {
            match find(data, at) {
                Ok(Found::Sized(header)) => {
                    self.steps += 1;
                    if header.kind == *b"mdat" {
                        self.register(header.body, header.end);
                    }
                    if header.end > data.len() {
                        break;
                    }
                    at = header.end;
                }
                Ok(Found::Open { kind, body }) => {
                    self.steps += 1;
                    if kind == *b"mdat" {
                        self.register(body, usize::MAX);
                    }
                    break;
                }
                Ok(Found::Cut) | Err(_) => break,
            }
        }
        self.scanned_to = self.scanned_to.max(at);
    }

    /// Both walks find the `mdat` boxes in file order: one already seen is the last or earlier.
    fn register(&mut self, start: usize, end: usize) {
        if self.mdats.last().is_none_or(|(seen, _)| *seen < start) {
            self.mdats.push((start, end));
        }
    }
}
