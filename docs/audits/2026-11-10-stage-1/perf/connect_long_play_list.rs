// Belongs in `crates/connect/src/lib.rs`, inside `mod tests` (after `paired_nodes_talk`): it uses
// that module's `Node::launch`, `Seen`, `lock` and `playing`, all private to the crate.
//
// Finding: a song played on a paired device from a long list sends the whole list
// (`crates/app-desktop/src/app.rs:3874-3880`, the phone's `Playback.kt:518-521`), where the
// device keeps 100 of them (`opium_core::queue::LIMIT`); past 32 MiB the device refuses the
// message (`wire.rs:163-165`) and the link drops, so the song never plays there. A file's song
// is about 580 bytes of JSON: the limit falls at about 57,600 songs listed.
//
// Fails today at "B never heard the song played on it": the link breaks on "message too long",
// then reconnects without the command. Passes once `Node::send` cuts a `PlayList` to what the
// queue keeps, the song asked for among it. A fix made in the apps instead (cutting before they
// call `send`) leaves this test failing: then it moves to where the cut is made.

/// A song played on the paired device from Songs, in a library of 70,000 songs: the command
/// reaches the device, the song asked for among what it gets.
#[test]
fn a_song_played_on_a_paired_device_from_a_long_list_reaches_it() {
    let dir = std::env::temp_dir().join(format!("connect-long-list-{}", std::process::id()));
    let (a_dir, b_dir) = (dir.join("a"), dir.join("b"));
    let (tx, rx) = std::sync::mpsc::channel();
    let a = Node::launch(
        "A".into(),
        DeviceKind::Computer,
        &a_dir,
        Kept::open(&a_dir, Vec::new),
        None,
        Box::new(|_| {}),
        false,
    );
    let b = Node::launch(
        "B".into(),
        DeviceKind::Phone,
        &b_dir,
        Kept::open(&b_dir, Vec::new),
        None,
        Box::new(move |e| {
            let _ = tx.send(e);
        }),
        false,
    );
    let key = [3; 32];
    lock(&a.inner.store).pair(b.me(), key);
    lock(&b.inner.store).pair(a.me(), key);
    let mut seen = Seen::new(b.me());
    seen.addrs = vec![SocketAddr::from(([127, 0, 0, 1], b.port))];
    lock(&a.inner.seen).insert(b.me().id, seen);
    a.publish(playing(0, false));
    a.inner.reach(&b.me().id);
    let deadline = Instant::now() + Duration::from_secs(5);
    while !b.peers().iter().any(|p| p.playing.is_some()) {
        assert!(Instant::now() < deadline, "B never heard A");
        std::thread::sleep(Duration::from_millis(20));
    }

    // As `local::track` lists a file: its path, its album's key, its kept cover, its quality.
    let local = opium_core::model::ProviderKind::Local;
    let tracks: Vec<Track> = (0..70_000u32)
        .map(|i| Track {
            id: opium_core::model::ItemId::new(
                local,
                format!("/srv/music/Some Artist/Some Album {}/{:02} A Song Title {i}.flac", i / 13, i % 13 + 1),
            ),
            title: format!("A Song Title {i}"),
            artists: vec!["Some Artist".into()],
            album: Some(format!("Some Album {}", i / 13)),
            album_id: Some(opium_core::model::ItemId::new(
                local,
                format!("album:some artist\u{1f}some album {}", i / 13),
            )),
            duration: Some(Duration::from_secs(200 + u64::from(i % 100))),
            track_number: Some(i % 13 + 1),
            disc_number: None,
            artwork: Some(opium_core::model::Artwork {
                url_template: format!(
                    "file:///var/lib/opium/local/covers/{:016x}.jpg",
                    u64::from(i / 13) * 0x9E37_79B9
                ),
                background: None,
            }),
            explicit: false,
            quality: Some(opium_core::model::AudioQuality {
                codec: opium_core::model::Codec::Flac,
                sample_rate: Some(44_100),
                bit_depth: Some(16),
                bitrate_kbps: Some(900),
            }),
            in_library: true,
            library_id: None,
        })
        .collect();
    let start = 41_234;
    let asked = tracks[start].id.clone();
    assert!(a.send(
        &b.me().id,
        Command::PlayList {
            tracks,
            start: start as u32
        }
    ));

    let deadline = Instant::now() + Duration::from_secs(20);
    let got = loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(left) {
            Ok(Event::Asked {
                command: Command::PlayList { tracks, start },
                ..
            }) => break Some((tracks, start)),
            Ok(_) => continue,
            Err(_) => break None,
        }
    };
    drop((a, b));
    let _ = std::fs::remove_dir_all(&dir);
    let (tracks, start) = got.expect("B never heard the song played on it");
    assert_eq!(tracks[start as usize].id, asked, "the song asked for");
    assert!(
        tracks.len() <= opium_core::queue::LIMIT,
        "{} songs sent where the queue keeps {}",
        tracks.len(),
        opium_core::queue::LIMIT
    );
}
