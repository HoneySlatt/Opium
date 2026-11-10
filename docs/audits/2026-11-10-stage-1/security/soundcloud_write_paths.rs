// Belongs in: crates/soundcloud/src/lib.rs, appended at the end of the file as a module of its own
// (it reads the private fields of `SoundCloud` and calls its private `like`).
//
// Run: cargo test -p soundcloud write_paths
//
// What it shows: an id that is not one of SoundCloud's (they are numbers) is put into the address
// of a change as it is, and `reqwest::Url::parse` resolves its `..` segments, so "unlike this
// track" is sent, with the user's token, to another endpoint of the API: here DELETE
// /playlists/555, which deletes the user's own playlist. Apple Music's writes refuse such an id
// (`apple_id`, crates/apple-music/src/lib.rs:304); SoundCloud's take it. Today the test fails on
// its first assertion; once SoundCloud's writes check their ids, it passes.

#[cfg(test)]
mod write_paths {
    use super::*;
    use opium_core::model::ProviderKind;
    use std::sync::Arc;

    /// Keeps each change it is handed, as `METHOD url`, and answers it as SoundCloud answers a
    /// like: 200, an empty object.
    #[derive(Default)]
    struct Recording(Mutex<Vec<String>>);

    #[async_trait]
    impl Browser for Recording {
        async fn send(&self, change: Change) -> Result<Answer> {
            self.0
                .lock()
                .unwrap()
                .push(format!("{} {}", change.method, change.url));
            Ok(Answer {
                status: 200,
                guard_header: false,
                body: "{}".into(),
            })
        }
    }

    /// Signed in as user 42, its changes sent through `browser`; no request leaves the test.
    fn signed_in(browser: Arc<Recording>) -> SoundCloud {
        SoundCloud {
            http: reqwest::Client::new(),
            client_id: RwLock::new("a".repeat(32)),
            tokens: Mutex::new(Tokens {
                oauth: "a-token".into(),
                refresh: None,
            }),
            on_tokens: Box::new(|_: &Tokens| {}),
            renewing: tokio::sync::Mutex::new(()),
            editing: tokio::sync::Mutex::new(()),
            rescraping: tokio::sync::Mutex::new(None),
            me: types::User {
                id: 42,
                ..Default::default()
            },
            stations: Mutex::default(),
            browser: Some(browser),
        }
    }

    /// The path a recorded change went to, its `..` resolved as the request resolves them.
    fn path_of(change: &str) -> String {
        let url = change.split_once(' ').unwrap().1;
        reqwest::Url::parse(url).unwrap().path().to_owned()
    }

    #[tokio::test]
    async fn unliking_a_track_changes_that_track_like_and_nothing_else() {
        let browser = Arc::new(Recording::default());
        let soundcloud = signed_in(browser.clone());
        let id = ItemId::new(ProviderKind::SoundCloud, "1/../../../../playlists/555");
        let unliked = soundcloud.like(FavouriteKind::Song, &id, false).await;
        let sent = browser.0.lock().unwrap().clone();
        for change in &sent {
            assert!(
                path_of(change).starts_with("/users/42/track_likes/"),
                "an unlike went elsewhere: {change}"
            );
        }
        // Refusing the id before anything is sent passes too.
        assert!(unliked.is_err() || !sent.is_empty());
    }

    #[tokio::test]
    async fn editing_a_playlist_changes_that_playlist_and_nothing_else() {
        let browser = Arc::new(Recording::default());
        let soundcloud = signed_in(browser.clone());
        let id = ItemId::new(ProviderKind::SoundCloud, "1/../../me/followings/7");
        let edited = soundcloud.edit_playlist(&id, "name", "").await;
        let sent = browser.0.lock().unwrap().clone();
        for change in &sent {
            assert!(
                path_of(change).starts_with("/playlists/"),
                "an edit went elsewhere: {change}"
            );
        }
        assert!(edited.is_err() || !sent.is_empty());
    }
}
