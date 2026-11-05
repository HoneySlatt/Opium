//! The samples' table: every sample of every fragment indexed, in file order, kept compact.
//!
//! The table is laid out in columns, so that no field is padded: 30 bytes a sample, plus its IV
//! bytes and its subsample runs in two shared arrays. What a sample shares with others (its
//! entry, key, scheme, pattern, IV size, constant IV) lives once in a [`Description`], which the
//! sample names by index.

use crate::{Pattern, Scheme};

/// A sample as the table gives it: its place in the file, and every input a CDM decrypts it with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sample<'a> {
    /// Where its bytes start, from the file's start.
    pub offset: usize,
    /// How many bytes it takes from `offset`; 0 for a sample with nothing to decrypt.
    pub size: usize,
    /// In its track's timescale.
    pub duration: u32,
    /// Where its `moof` starts.
    pub fragment: usize,
    /// The index of its sample entry in its track, from 0.
    pub entry: u32,
    /// The id of the key that decrypts it, from its `seig` group or else its entry's `tenc`:
    /// all zero when the keys are named elsewhere (Spatial Audio), and for a clear sample.
    pub key_id: [u8; 16],
    /// Its IV, padded to 16 bytes with zeros: an 8-byte IV is the upper half of the counter
    /// block (23001-7 §9.1).
    pub iv: [u8; 16],
    /// How many bytes of `iv` the file gave: 8 or 16, or 0 for a sample with no IV.
    pub iv_size: u8,
    /// Its (clear, protected) byte runs, in order; empty when it is protected whole.
    pub subsamples: &'a [(u32, u32)],
    /// Whether its bytes are encrypted; a clear sample needs nothing of the CDM.
    pub protected: bool,
    /// The scheme its entry's `schm` names; a clear sample has none, and says `Cenc`.
    pub scheme: Scheme,
    /// The crypt:skip pattern of its protected ranges, from its `seig` group or else its
    /// entry's `tenc`; 0:0 when there is none, as for a clear sample.
    pub pattern: Pattern,
}

/// What a run of samples shares: their entry and the protection their `tenc` or `seig` gives.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) struct Description {
    pub entry: u32,
    pub protected: bool,
    pub scheme: Scheme,
    pub pattern: Pattern,
    /// Bytes of IV each sample carries: 0 when `constant_iv` serves.
    pub iv_size: u8,
    pub key_id: [u8; 16],
    pub constant_iv: [u8; 16],
    pub constant_iv_size: u8,
}

impl Description {
    /// A clear sample of `entry`: nothing of it needs the CDM.
    pub fn clear(entry: u32) -> Self {
        Self {
            entry,
            protected: false,
            scheme: Scheme::Cenc,
            pattern: Pattern::default(),
            iv_size: 0,
            key_id: [0; 16],
            constant_iv: [0; 16],
            constant_iv_size: 0,
        }
    }
}

/// A fragment indexed: where its `moof` starts and which sample is its first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Fragment {
    pub start: usize,
    pub first_sample: usize,
}

/// A sample's 32-bit fields, together so that the table stays a few columns.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Fields {
    size: u32,
    duration: u32,
    /// Where the sample's IV starts in the shared IVs; unused under a constant IV.
    iv_at: u32,
    /// Where the sample's subsample runs start in the shared runs.
    runs_at: u32,
}

/// Every sample indexed so far, in file order.
///
/// Invariant: the per-sample columns have the same length; every index a sample holds into the
/// shared arrays and into the descriptions is in range.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SampleTable {
    offsets: Vec<usize>,
    fields: Vec<Fields>,
    description_of: Vec<u32>,
    run_counts: Vec<u16>,
    ivs: Vec<u8>,
    runs: Vec<(u32, u32)>,
    descriptions: Vec<Description>,
    fragments: Vec<Fragment>,
}

/// The lengths of the table's growing arrays at one moment, to go back to.
#[derive(Clone, Copy)]
pub(crate) struct Mark {
    pub samples: usize,
    ivs: usize,
    runs: usize,
    fragments: usize,
}

impl SampleTable {
    /// How many samples the table holds.
    pub fn len(&self) -> usize {
        self.offsets.len()
    }

    /// Whether the table holds no sample: no fragment indexed yet, or none with samples.
    pub fn is_empty(&self) -> bool {
        self.offsets.is_empty()
    }

    /// Sample `index`, in file order.
    pub fn get(&self, index: usize) -> Option<Sample<'_>> {
        (index < self.len()).then(|| self.sample(index))
    }

    /// Every sample, in file order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = Sample<'_>> + '_ {
        (0..self.len()).map(move |index| self.sample(index))
    }

    /// Sample `index`, which must be below `len`.
    fn sample(&self, index: usize) -> Sample<'_> {
        let description = &self.descriptions[self.description_of[index] as usize];
        let fields = self.fields[index];
        let mut iv = [0; 16];
        let iv_size = if description.iv_size == 0 {
            iv = description.constant_iv;
            description.constant_iv_size
        } else {
            let at = fields.iv_at as usize;
            let size = description.iv_size as usize;
            iv[..size].copy_from_slice(&self.ivs[at..at + size]);
            description.iv_size
        };
        let runs_at = fields.runs_at as usize;
        let runs_end = runs_at + self.run_counts[index] as usize;
        Sample {
            offset: self.offsets[index],
            size: fields.size as usize,
            duration: fields.duration,
            fragment: self.fragment_of(index).start,
            entry: description.entry,
            key_id: description.key_id,
            iv,
            iv_size,
            subsamples: &self.runs[runs_at..runs_end],
            protected: description.protected,
            scheme: description.scheme,
            pattern: description.pattern,
        }
    }

    /// The fragment sample `index` belongs to: the last one starting at or before it.
    fn fragment_of(&self, index: usize) -> Fragment {
        let after = self
            .fragments
            .partition_point(|fragment| fragment.first_sample <= index);
        self.fragments[after - 1]
    }

    /// The fragments indexed, in file order.
    pub(crate) fn fragments(&self) -> &[Fragment] {
        &self.fragments
    }

    pub(crate) fn mark(&self) -> Mark {
        Mark {
            samples: self.len(),
            ivs: self.ivs.len(),
            runs: self.runs.len(),
            fragments: self.fragments.len(),
        }
    }

    /// Drops everything added since `mark`; the descriptions stay, they are shared.
    pub(crate) fn truncate(&mut self, mark: Mark) {
        self.offsets.truncate(mark.samples);
        self.fields.truncate(mark.samples);
        self.description_of.truncate(mark.samples);
        self.run_counts.truncate(mark.samples);
        self.ivs.truncate(mark.ivs);
        self.runs.truncate(mark.runs);
        self.fragments.truncate(mark.fragments);
    }

    /// Makes room for `count` more samples, amortised.
    pub(crate) fn reserve(&mut self, count: usize) {
        self.offsets.reserve(count);
        self.fields.reserve(count);
        self.description_of.reserve(count);
        self.run_counts.reserve(count);
    }

    /// Makes room, exactly, for `rest` more bytes of file like the `span` bytes that hold the
    /// samples so far: as many samples, IV bytes and subsample runs per byte, and an eighth more
    /// for a file whose density varies, so that it is not outgrown by a little at its end. Each
    /// array is given room for at most `rest` more items: the input justifies what it reserves.
    pub(crate) fn reserve_for_rest(&mut self, span: usize, rest: usize) {
        if span == 0 {
            return;
        }
        let like = |count: usize| {
            let estimate = count as u128 * rest as u128 / span as u128;
            (estimate + estimate / 8).min(rest as u128) as usize
        };
        let samples = like(self.len());
        self.offsets.reserve_exact(samples);
        self.fields.reserve_exact(samples);
        self.description_of.reserve_exact(samples);
        self.run_counts.reserve_exact(samples);
        self.ivs.reserve_exact(like(self.ivs.len()));
        self.runs.reserve_exact(like(self.runs.len()));
    }

    /// Adds a sample whose IV, runs and description come later ([`Self::set_description`],
    /// [`Self::set_aux`]).
    pub(crate) fn push(&mut self, offset: usize, size: u32, duration: u32) {
        self.offsets.push(offset);
        self.fields.push(Fields {
            size,
            duration,
            ..Fields::default()
        });
        self.description_of.push(0);
        self.run_counts.push(0);
    }

    pub(crate) fn offset(&self, index: usize) -> usize {
        self.offsets[index]
    }

    pub(crate) fn size(&self, index: usize) -> u32 {
        self.fields[index].size
    }

    pub(crate) fn description(&self, index: usize) -> &Description {
        &self.descriptions[self.description_of[index] as usize]
    }

    pub(crate) fn set_description(&mut self, index: usize, description: u32) {
        self.description_of[index] = description;
    }

    /// Where sample `index`'s IV and runs are: set once they are pushed.
    pub(crate) fn set_aux(&mut self, index: usize, iv_at: u32, runs_at: u32, run_count: u16) {
        self.fields[index].iv_at = iv_at;
        self.fields[index].runs_at = runs_at;
        self.run_counts[index] = run_count;
    }

    /// Adds `iv`'s bytes to the shared IVs; where they start, if the array can still be indexed.
    pub(crate) fn push_iv(&mut self, iv: &[u8]) -> Option<u32> {
        let at = u32::try_from(self.ivs.len()).ok()?;
        self.ivs.extend_from_slice(iv);
        Some(at)
    }

    /// Where the next subsample run goes, if the array can still be indexed.
    pub(crate) fn runs_end(&self) -> Option<u32> {
        u32::try_from(self.runs.len()).ok()
    }

    pub(crate) fn reserve_runs(&mut self, count: usize) {
        self.runs.reserve(count);
    }

    pub(crate) fn push_run(&mut self, clear: u32, protected: u32) {
        self.runs.push((clear, protected));
    }

    pub(crate) fn push_fragment(&mut self, fragment: Fragment) {
        self.fragments.push(fragment);
    }

    /// Adds a description; its index. Callers share equal descriptions themselves.
    pub(crate) fn push_description(&mut self, description: Description) -> Option<u32> {
        let index = u32::try_from(self.descriptions.len()).ok()?;
        self.descriptions.push(description);
        Some(index)
    }

    /// Gives back the room the arrays grew by: the file is whole.
    pub(crate) fn shrink_to_fit(&mut self) {
        self.offsets.shrink_to_fit();
        self.fields.shrink_to_fit();
        self.description_of.shrink_to_fit();
        self.run_counts.shrink_to_fit();
        self.ivs.shrink_to_fit();
        self.runs.shrink_to_fit();
        self.descriptions.shrink_to_fit();
        self.fragments.shrink_to_fit();
    }
}
