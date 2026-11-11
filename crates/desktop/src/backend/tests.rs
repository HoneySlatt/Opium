//! The place where requests wait on a connect (`Connects`), the sign-outs
//! that make a connect stale (`SignOuts`), and which requests wait at all.
//! The backend's loop itself opens the keyring, loads the CDM and reads the
//! profile's folders, so it is run on a throwaway profile rather than here.

use opium_core::model::{ItemId, ProviderKind};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::oneshot;

use super::{
    Arrived, Asked, Connection, Connects, Ended, Request, SignOuts,
    keep_soundcloud_cookie_while_signed_in, list_copy, waits_on,
};
use crate::SearchScope;

const APPLE: ProviderKind = ProviderKind::AppleMusic;
const NAVIDROME: ProviderKind = ProviderKind::Navidrome;

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("a runtime for the test")
}

fn connects() -> (Connects, UnboundedReceiver<Arrived>, SignOuts) {
    let (arrived, came) = tokio::sync::mpsc::unbounded_channel();
    let sign_outs = SignOuts::default();
    (Connects::new(arrived, sign_outs.clone()), came, sign_outs)
}

/// A connect of `provider` that ends, unconnected, once `finish` is sent.
fn pending_connect(connects: &mut Connects, provider: ProviderKind) -> oneshot::Sender<()> {
    let (finish, finished) = oneshot::channel::<()>();
    connects.start(provider, true, async move {
        let _ = finished.await;
        let connection = match provider {
            ProviderKind::AppleMusic => Connection::Apple(None),
            ProviderKind::SoundCloud => Connection::SoundCloud(None),
            _ => Connection::Navidrome(None),
        };
        Ended {
            connection,
            told: None,
        }
    });
    finish
}

fn lyrics_of(provider: ProviderKind) -> Request {
    Request::Lyrics(ItemId::new(provider, "1"))
}

fn handed_back(connects: &mut Connects) -> Vec<Asked> {
    std::iter::from_fn(|| connects.next()).collect()
}

#[test]
fn what_waits_on_a_connect_is_handed_back_in_order_once_it_ends() {
    runtime().block_on(async {
        let (mut connects, mut came, _) = connects();
        let finish = pending_connect(&mut connects, APPLE);
        assert!(connects.under_way(APPLE));
        assert!(!connects.under_way(NAVIDROME));
        connects.hold(APPLE, lyrics_of(APPLE), None);
        connects.hold(APPLE, Request::LibraryAlbums, Some(APPLE));
        assert!(connects.next().is_none(), "nothing before the connect ends");

        let _ = finish.send(());
        let arrived = came.recv().await.expect("the connect comes back");
        assert_eq!(arrived.provider, APPLE);
        assert!(arrived.load);
        assert!(matches!(
            arrived.ended,
            Some(Ended {
                connection: Connection::Apple(None),
                told: None
            })
        ));
        connects.ended(arrived.provider, arrived.sign_outs);

        assert!(!connects.under_way(APPLE));
        let back = handed_back(&mut connects);
        assert_eq!(back.len(), 2);
        assert!(matches!(back[0].request, Request::Lyrics(_)) && back[0].only.is_none());
        assert!(matches!(back[1].request, Request::LibraryAlbums) && back[1].only == Some(APPLE));
    });
}

#[test]
fn what_waits_is_handed_back_once_every_connect_of_its_service_has_ended() {
    runtime().block_on(async {
        let (mut connects, mut came, _) = connects();
        let first = pending_connect(&mut connects, NAVIDROME);
        let second = pending_connect(&mut connects, NAVIDROME);
        connects.hold(NAVIDROME, lyrics_of(NAVIDROME), None);

        let _ = first.send(());
        let arrived = came.recv().await.expect("the first comes back");
        connects.ended(arrived.provider, arrived.sign_outs);
        assert!(connects.under_way(NAVIDROME));
        assert!(
            connects.next().is_none(),
            "another connect is still under way"
        );

        let _ = second.send(());
        let arrived = came.recv().await.expect("the second comes back");
        connects.ended(arrived.provider, arrived.sign_outs);
        assert_eq!(handed_back(&mut connects).len(), 1);
    });
}

#[test]
fn a_service_turned_off_or_signed_out_of_hands_back_what_waits_at_once() {
    runtime().block_on(async {
        let (mut connects, mut came, _) = connects();
        let finish = pending_connect(&mut connects, APPLE);
        connects.hold(APPLE, lyrics_of(APPLE), None);
        connects.hold(NAVIDROME, lyrics_of(NAVIDROME), None);

        connects.release(APPLE);
        let back = handed_back(&mut connects);
        assert_eq!(back.len(), 1, "only what waits on that service");
        assert!(matches!(&back[0].request, Request::Lyrics(id) if id.provider == APPLE));

        // Its connect ending later hands back nothing twice.
        let _ = finish.send(());
        let arrived = came.recv().await.expect("the connect comes back");
        connects.ended(arrived.provider, arrived.sign_outs);
        assert!(connects.next().is_none());
    });
}

#[test]
fn a_connect_that_panics_still_comes_back_so_that_nothing_waits_for_ever() {
    runtime().block_on(async {
        let (mut connects, mut came, _) = connects();
        // Without the panic hook, so that the test's output stays quiet.
        connects.start(APPLE, false, async {
            std::panic::resume_unwind(Box::new("a connect that failed badly"))
        });
        connects.hold(APPLE, lyrics_of(APPLE), None);
        let arrived = came.recv().await.expect("the connect comes back");
        assert!(arrived.ended.is_none(), "it changes nothing");
        connects.ended(arrived.provider, arrived.sign_outs);
        assert_eq!(handed_back(&mut connects).len(), 1);
    });
}

#[test]
fn a_connect_started_before_a_sign_out_comes_back_counted_before_it() {
    runtime().block_on(async {
        let (mut connects, mut came, sign_outs) = connects();
        let before = pending_connect(&mut connects, APPLE);
        sign_outs.sign_out(APPLE);
        let after = pending_connect(&mut connects, APPLE);

        let _ = before.send(());
        let stale = came.recv().await.expect("the first comes back");
        let _ = after.send(());
        let fresh = came.recv().await.expect("the second comes back");
        // The loop drops the one whose count is no longer the service's.
        assert_ne!(stale.sign_outs, sign_outs.count(APPLE));
        assert_eq!(fresh.sign_outs, sign_outs.count(APPLE));
    });
}

#[test]
fn tokens_are_kept_only_while_the_sign_in_they_renew_is_there() {
    let sign_outs = SignOuts::default();
    let since = sign_outs.count(ProviderKind::SoundCloud);
    let mut kept = 0;
    sign_outs.unless_signed_out_since(ProviderKind::SoundCloud, since, || kept += 1);
    assert_eq!(kept, 1);

    // Another service's sign-out changes nothing.
    sign_outs.sign_out(APPLE);
    sign_outs.unless_signed_out_since(ProviderKind::SoundCloud, since, || kept += 1);
    assert_eq!(kept, 2);

    sign_outs.sign_out(ProviderKind::SoundCloud);
    sign_outs.unless_signed_out_since(ProviderKind::SoundCloud, since, || kept += 1);
    assert_eq!(kept, 2, "renewed after the sign-out: not written back");

    // A sign-in made since keeps its own.
    let since = sign_outs.count(ProviderKind::SoundCloud);
    sign_outs.unless_signed_out_since(ProviderKind::SoundCloud, since, || kept += 1);
    assert_eq!(kept, 3);
}

/// SoundCloud's bot-guard cookie, renewed by the answer to a change still
/// under way as the user signs out, is not written back once the sign-out
/// has deleted it: as the tokens (the test before), the sign-in is gone.
#[test]
fn the_bot_guards_cookie_is_kept_only_while_the_sign_in_it_was_renewed_for_is_there() {
    let dir = std::env::temp_dir().join(format!("desktop-datadome-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("soundcloud").join("datadome");
    let sign_outs = SignOuts::default();
    let keep = keep_soundcloud_cookie_while_signed_in(&sign_outs, path.clone());
    keep("first");
    let first = std::fs::read(&path).ok();

    // Another service's sign-out changes nothing.
    sign_outs.sign_out(APPLE);
    keep("second");
    let second = std::fs::read(&path).ok();

    // Signed out of: counted, then the cookie deleted, as `forget_sign_in` does.
    sign_outs.sign_out(ProviderKind::SoundCloud);
    let _ = std::fs::remove_file(&path);
    keep("renewed after the sign-out");
    let written_back = path.exists();

    // A sign-in made since keeps its own.
    keep_soundcloud_cookie_while_signed_in(&sign_outs, path.clone())("a new sign-in's");
    let new = std::fs::read(&path).ok();
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(first.as_deref(), Some(&b"first"[..]));
    assert_eq!(second.as_deref(), Some(&b"second"[..]));
    assert!(!written_back, "renewed after the sign-out: written back");
    assert_eq!(new.as_deref(), Some(&b"a new sign-in's"[..]));
}

#[test]
fn what_one_service_answers_waits_on_it_and_lists_of_several_are_asked_in_parts() {
    let fetch = |track: ItemId| Request::Fetch {
        token: 1,
        track,
        fallback: None,
        original: false,
        spatial: false,
    };
    let warm = Request::Warm {
        token: 2,
        track: ItemId::new(APPLE, "1"),
        fallback: None,
        original: false,
        spatial: false,
    };
    assert_eq!(waits_on(&fetch(ItemId::new(APPLE, "1"))), Some(APPLE));
    assert_eq!(waits_on(&lyrics_of(NAVIDROME)), Some(NAVIDROME));
    assert_eq!(
        waits_on(&Request::Home(ProviderKind::SoundCloud)),
        Some(ProviderKind::SoundCloud)
    );
    assert_eq!(
        waits_on(&Request::Motion {
            url: String::new(),
            size: 1
        }),
        Some(APPLE)
    );
    // Warmed ahead: answered at once, fetched again if played, so that an
    // old warming handed back never calls off a newer one.
    assert_eq!(waits_on(&warm), None);
    // A file never waits: the files have no connect.
    assert_eq!(
        waits_on(&fetch(ItemId::new(ProviderKind::Local, "/a.flac"))),
        Some(ProviderKind::Local)
    );

    let search = |source| Request::Search(SearchScope::Library, source, "query".into());
    assert_eq!(waits_on(&search(Some(NAVIDROME))), Some(NAVIDROME));
    assert!(list_copy(&search(Some(NAVIDROME))).is_none());
    assert_eq!(waits_on(&search(None)), None);
    assert!(matches!(
        list_copy(&search(None)),
        Some(Request::Search(SearchScope::Library, None, query)) if query == "query"
    ));
    for list in [
        Request::LibraryAlbums,
        Request::LibraryArtists,
        Request::RecentlyAdded,
        Request::NewestSongs,
        Request::Playlists,
    ] {
        assert_eq!(waits_on(&list), None);
        assert!(list_copy(&list).is_some());
    }
    // The library shows what was kept at once, and loads each service's own
    // as it connects.
    assert!(waits_on(&Request::Library).is_none() && list_copy(&Request::Library).is_none());
}

/// A song of `provider` to download.
fn song(provider: ProviderKind, id: &str) -> opium_core::model::Track {
    opium_core::model::Track {
        id: ItemId::new(provider, id),
        title: id.into(),
        artists: Vec::new(),
        album: None,
        album_id: None,
        duration: None,
        track_number: None,
        disc_number: None,
        artwork: None,
        explicit: false,
        quality: None,
        in_library: false,
        library_id: None,
    }
}

#[test]
fn a_connect_started_before_a_sign_out_holds_nothing_once_signed_in_again() {
    runtime().block_on(async {
        let (mut connects, mut came, sign_outs) = connects();
        // A start connect that stalls; the user signs out, then in again.
        let stale = pending_connect(&mut connects, APPLE);
        sign_outs.sign_out(APPLE);
        connects.release(APPLE);
        assert!(
            !connects.under_way(APPLE),
            "the stale connect waits on nothing"
        );
        let fresh = pending_connect(&mut connects, APPLE);
        connects.hold(APPLE, lyrics_of(APPLE), None);

        let _ = fresh.send(());
        let arrived = came.recv().await.expect("the new connect comes back");
        connects.ended(arrived.provider, arrived.sign_outs);
        assert_eq!(
            handed_back(&mut connects).len(),
            1,
            "answered once the new connect ends, the stale one still under way"
        );
        drop(stale);
    });
}

#[test]
fn songs_waiting_to_be_downloaded_are_called_off_by_a_removal_or_a_sign_out() {
    runtime().block_on(async {
        let (mut connects, _came, _) = connects();
        let _apple = pending_connect(&mut connects, APPLE);
        let _navidrome = pending_connect(&mut connects, NAVIDROME);
        let download = |provider, ids: &[&str]| Request::Download {
            tracks: ids.iter().map(|id| song(provider, id)).collect(),
            original: false,
        };
        connects.hold(APPLE, download(APPLE, &["1", "2"]), None);
        connects.hold(APPLE, lyrics_of(APPLE), None);
        connects.hold(NAVIDROME, download(NAVIDROME, &["3"]), None);

        connects.remove_downloads(&[ItemId::new(APPLE, "1"), ItemId::new(NAVIDROME, "3")]);
        assert!(
            connects.forget_downloads(NAVIDROME).is_empty(),
            "its one song was removed"
        );
        connects.release(APPLE);
        connects.release(NAVIDROME);
        let back = handed_back(&mut connects);
        assert_eq!(
            back.len(),
            2,
            "Apple's song 2 and its lyrics, nothing of Navidrome's"
        );
        assert!(matches!(&back[0].request,
            Request::Download { tracks, .. } if tracks.len() == 1 && tracks[0].id.id == "2"));

        let (mut connects, _came, _) = super::tests::connects();
        let _apple = pending_connect(&mut connects, APPLE);
        connects.hold(APPLE, download(APPLE, &["1"]), None);
        connects.hold(APPLE, lyrics_of(APPLE), None);
        assert_eq!(
            connects.forget_downloads(APPLE),
            [ItemId::new(APPLE, "1")],
            "the songs called off, to be told so"
        );
        connects.release(APPLE);
        let back = handed_back(&mut connects);
        assert_eq!(back.len(), 1, "a sign-out calls the songs off");
        assert!(matches!(back[0].request, Request::Lyrics(_)));
    });
}

/// A secret (Apple's user token, SoundCloud's tokens and cookie, Navidrome's sign-in, the
/// settings) is written readable by the user alone, in a folder only the user opens, and
/// replaced whole (through `opium_core::files::write_whole`): AGENTS.md's "Tokens in user-only
/// files (`desktop::backend::write_private`)", which no test checked.
///
/// Shown to catch `write_private` replaced by `Ok(())`, and `private_dir` without its
/// `set_permissions`.
#[test]
fn a_secret_is_written_for_the_user_alone_and_whole() {
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    use super::write_private;

    let dir = std::env::temp_dir().join(format!("desktop-private-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let folder = dir.join("apple-music");
    let path = folder.join("media-user-token");
    let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;

    write_private(&path, b"first").unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"first");
    assert_eq!(mode(&path), 0o600);
    assert_eq!(mode(&folder), 0o700);

    // Again: replaced whole, nothing left beside it.
    write_private(&path, b"second, and longer").unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"second, and longer");
    assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 1);

    // A folder opened to others meanwhile is the user's alone again.
    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o755)).unwrap();
    write_private(&path, b"third").unwrap();
    assert_eq!(mode(&folder), 0o700);

    std::fs::remove_dir_all(&dir).unwrap();
}
