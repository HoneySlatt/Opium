//! Encrypted fragmented MP4 (CENC), as Apple Music and SoundCloud serve it, and a stream that
//! reads as the plain MP4 while a CDM decrypts it in the background ([`AudioStream`]).
//!
//! A file is one movie box ([`Movie`]) then fragments, each a `moof` and its `mdat`. The movie
//! says how each track's samples are protected (ISO/IEC 23001-7); the fragments say where each
//! sample is and what decrypts it (ISO/IEC 14496-12 §8.8). [`index`] reads a whole file;
//! [`Indexer`] reads one as it downloads, fragment by whole fragment; [`make_clear`] then makes
//! the file read as a plain MP4, changing only box types and counts.
//!
//! No input makes this crate panic, loop, recurse without bound or allocate more than the input
//! justifies: every size and offset is checked, every count against the bytes left. Errors name
//! the box, where it starts and what was wrong.

mod boxes;
mod claims;
mod clear;
mod error;
mod fragment;
mod indexer;
mod movie;
mod stream;
mod table;

#[cfg(test)]
mod tests;

pub use clear::{last_sample_duration, make_clear, write_key_ids};
pub use error::Error;
pub use indexer::Indexer;
pub use movie::{Movie, Pattern, Protection, Pssh, SampleEntry, Scheme, Track, TrackDefaults};
pub use stream::{AudioStream, Decrypt};
pub use table::{Sample, SampleTable};

/// Indexes a whole file: its movie and every sample of every fragment, in file order.
///
/// # Errors
///
/// A file without a movie box, or whose last box is cut short; and every error of
/// [`Movie::read`] and [`Indexer::advance`].
pub fn index(data: &[u8]) -> Result<(Movie, SampleTable), Error> {
    index_counting(data).0
}

/// [`index`], with how many boxes, samples and entries were read: tests bound the work done by
/// the input's size.
pub(crate) fn index_counting(data: &[u8]) -> (Result<(Movie, SampleTable), Error>, usize) {
    let mut steps = 0;
    let result = movie::read(data, true, &mut steps).and_then(|movie| {
        let movie = movie
            .ok_or_else(|| Error::new(0, *b"moov", "is missing: the file has no movie box"))?;
        let mut indexer = Indexer::new(&movie);
        let finished = indexer.finish(data);
        steps += indexer.steps();
        finished.map(|()| (movie, indexer.into_samples()))
    });
    (result, steps)
}
