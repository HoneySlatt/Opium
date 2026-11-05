//! A whole file indexed, then made clear: no panic, every sample inside the file, and the
//! cleared file the same length.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok((movie, samples)) = cenc::index(data) else {
        return;
    };
    for sample in samples.iter() {
        assert!(sample.offset + sample.size <= data.len());
        assert!(sample.fragment < data.len());
        let total: u64 = sample.subsamples.iter().map(|(c, p)| u64::from(*c) + u64::from(*p)).sum();
        assert!(sample.subsamples.is_empty() || total == sample.size as u64);
    }
    let mut buf = data.to_vec();
    cenc::make_clear(&mut buf, &movie, &samples);
    assert_eq!(buf.len(), data.len());
    let mut again = buf.clone();
    cenc::make_clear(&mut again, &movie, &samples);
    assert_eq!(again, buf, "making clear twice changes nothing more");
    let _ = cenc::last_sample_duration(data);
    let mut init = data.to_vec();
    cenc::write_key_ids(&mut init, &[[7; 16], [8; 16]]);
});
