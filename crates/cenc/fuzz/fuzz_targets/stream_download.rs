//! A download fed to `AudioStream` in pieces cut where the input says, its samples "decrypted" by
//! XOR, then read whole: no panic and no hang, and a file that indexes whole reads as that file
//! made clear with every protected sample XORed.

#![no_main]

use std::io::Read;
use std::time::Duration;

use libfuzzer_sys::fuzz_target;

/// What the stand-in for the CDM XORs each byte of a protected sample with.
const XOR: u8 = 0x5A;

/// How long each sample plays, as for AAC at 44.1 kHz: it sets how far ahead the stream decrypts.
const SAMPLE_DURATION: Duration = Duration::from_micros(23_220);

fuzz_target!(|data: &[u8]| {
    // The first byte says how many cuts follow (up to 7), each a 16-bit offset into the file.
    let Some((&count, rest)) = data.split_first() else {
        return;
    };
    let count = usize::from(count & 7);
    if rest.len() < 2 * count {
        return;
    }
    let (cuts, file) = rest.split_at(2 * count);
    let mut cuts: Vec<usize> = cuts
        .chunks(2)
        .map(|c| usize::from(u16::from_be_bytes([c[0], c[1]])).min(file.len()))
        .collect();
    cuts.sort_unstable();
    cuts.push(file.len());

    let decrypt: cenc::Decrypt = Box::new(|bytes: &mut [u8], sample: &cenc::Sample<'_>| {
        assert_eq!(
            bytes.len(),
            sample.size,
            "the stream hands over the sample's bytes"
        );
        bytes.iter_mut().for_each(|byte| *byte ^= XOR);
        Ok(())
    });
    let (mut stream, feed) =
        cenc::AudioStream::downloading(file.len() as u64, SAMPLE_DURATION, decrypt);
    let mut from = 0;
    for cut in cuts {
        feed.push(&file[from..cut]);
        from = cut;
    }
    feed.end(None);

    let mut read = Vec::new();
    let result = stream.read_to_end(&mut read);
    assert!(read.len() <= file.len(), "never more than the file");
    if let (Ok(_), Ok((movie, samples))) = (result, cenc::index(file)) {
        let mut expected = file.to_vec();
        cenc::make_clear(&mut expected, &movie, &samples);
        for sample in samples.iter().filter(|s| s.protected && s.size > 0) {
            expected[sample.offset..sample.offset + sample.size]
                .iter_mut()
                .for_each(|byte| *byte ^= XOR);
        }
        assert!(
            read == expected,
            "the stream reads as the file made clear and decrypted"
        );
    }
});
