//! The tests of the specification's §13: the box layouts of §2 to §5, the malformed cases of §9,
//! the mutation test, and making a file clear (§6). The allocation and budget tests, which need
//! a global allocator of their own, are in `tests/allocation.rs`.

mod build;
mod clear;
mod layouts;
mod malformed;
mod mutation;
mod other;

use crate::{Error, Indexer, Movie, Sample, SampleTable, Scheme};
use build::Expected;

/// `sample` in the shape the builder predicts.
fn as_expected(sample: Sample<'_>) -> Expected {
    Expected {
        offset: sample.offset,
        size: sample.size,
        duration: sample.duration,
        fragment: sample.fragment,
        entry: sample.entry,
        key_id: sample.key_id,
        iv: sample.iv,
        iv_size: sample.iv_size,
        subsamples: sample.subsamples.to_vec(),
        protected: sample.protected,
        scheme: scheme_name(sample.scheme),
        pattern: (sample.pattern.crypt, sample.pattern.skip),
    }
}

fn scheme_name(scheme: Scheme) -> [u8; 4] {
    match scheme {
        Scheme::Cenc => *b"cenc",
        Scheme::Cens => *b"cens",
        Scheme::Cbc1 => *b"cbc1",
        Scheme::Cbcs => *b"cbcs",
    }
}

/// Indexes `bytes` fed to an [`Indexer`] `chunk` bytes at a time, as a download would.
fn stream_index(bytes: &[u8], chunk: usize) -> Result<(Movie, SampleTable), Error> {
    let mut arrived: Option<(Movie, Indexer)> = None;
    let mut fed = 0;
    while fed < bytes.len() {
        fed = (fed + chunk).min(bytes.len());
        let data = &bytes[..fed];
        if arrived.is_none() {
            match Movie::read(data)? {
                Some(movie) => arrived = Some((Indexer::new(&movie), movie)).map(|(i, m)| (m, i)),
                None => continue,
            }
        }
        let (_, indexer) = arrived.as_mut().expect("the movie has arrived");
        let ready = indexer.advance(data)?;
        assert!(
            ready <= data.len(),
            "the file is whole only as far as it has come"
        );
    }
    let (movie, mut indexer) =
        arrived.ok_or_else(|| Error::new(0, *b"moov", "is missing: the file has no movie box"))?;
    indexer.finish(bytes)?;
    Ok((movie, indexer.into_samples()))
}

/// Indexes `bytes` whole and streamed in `chunk`-byte pieces, which must agree.
fn index_both_ways(bytes: &[u8], chunk: usize) -> Result<(Movie, SampleTable), Error> {
    let whole = crate::index(bytes);
    let streamed = stream_index(bytes, chunk);
    assert_eq!(whole, streamed, "whole and streamed indexing agree");
    whole
}
