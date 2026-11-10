// Belongs in crates/cenc/src/stream.rs, inside `mod tests` (it uses that
// module's `track` and `xor`). Linux only: it counts the decrypt thread's
// sleeps in /proc. Run alone, so that no other test's decrypt thread starts
// meanwhile:
//
//     cargo test -p cenc a_stream_nobody_reads_lets_its_decrypt_thread_sleep
//
// Today it fails: the thread waits with `wait_timeout(state, NAP)`, NAP being
// 250 ms (stream.rs:91, 372-375), so it wakes about 8 times in the 2 s.

/// The ids of this process's threads named `name`.
fn threads_named(name: &str) -> Vec<String> {
    std::fs::read_dir("/proc/self/task")
        .expect("this process's threads")
        .flatten()
        .filter(|thread| {
            std::fs::read_to_string(thread.path().join("comm"))
                .is_ok_and(|comm| comm.trim_end() == name)
        })
        .map(|thread| thread.file_name().to_string_lossy().into_owned())
        .collect()
}

/// How many times thread `id` of this process went to sleep.
fn sleeps(id: &str) -> u64 {
    std::fs::read_to_string(format!("/proc/self/task/{id}/status"))
        .expect("the thread's status")
        .lines()
        .find_map(|line| line.strip_prefix("voluntary_ctxt_switches:"))
        .and_then(|count| count.trim().parse().ok())
        .expect("its voluntary context switches")
}

/// A song paused: nobody reads, the decrypt thread is its 30 seconds ahead,
/// and nothing changes. It sleeps until something does, rather than look
/// again four times a second.
#[test]
fn a_stream_nobody_reads_lets_its_decrypt_thread_sleep() {
    // 40 samples of a second: the thread decrypts the first 31, then waits.
    let (_, data, samples) = track(1, 40, 50);
    let before = threads_named("decrypt");
    let stream = AudioStream::decrypting(data, samples, Duration::from_secs(1), xor());
    std::thread::sleep(Duration::from_millis(500));
    let ours: Vec<String> = threads_named("decrypt")
        .into_iter()
        .filter(|id| !before.contains(id))
        .collect();
    assert_eq!(ours.len(), 1, "one decrypt thread started: {ours:?}");
    let at_first = sleeps(&ours[0]);
    std::thread::sleep(Duration::from_secs(2));
    let woke = sleeps(&ours[0]) - at_first;
    drop(stream);
    assert!(
        woke <= 1,
        "the decrypt thread woke {woke} times in 2 s with nothing to do"
    );
}
