    // Belongs in: crates/apple-music/src/lib.rs, module `http_tests` (it uses `apple_server`,
    // `apple_music`, `json_reply` and `json!`, which that module imports).
    //
    // FAILS at 46e7a5a: it proves a defect. `AppleMusic::each_page_in_turn` (lib.rs:1078) stops only
    // when an answer has no `next`, or when the offset `next` names reaches `limit`, which is
    // `usize::MAX` for every library list (lib.rs:783, 889, 894, 947, 955, 991). An answer without
    // `meta.total` whose `next` names the same offset, or an earlier one, is asked for again, then
    // again, for as long as Apple keeps answering, with no pause between requests: the app never
    // finishes reading the library, and the account sends Apple requests without end.
    //
    // Expected once fixed: the list ends (or fails) after a bounded number of requests, for example
    // by stopping when `next` doesn't move past the offset already reached. The bound of 3 below
    // leaves room for one request more than the fix needs.
    //
    // It also gives the mutation run's timeouts in `each_page_in_turn` (5) and `each_page` (3) a
    // cause: with a guard, those mutants would end with a wrong answer instead of hanging.

    /// A list whose `next` never moves on: asked a few times at most, not for ever.
    #[tokio::test]
    async fn a_next_page_that_never_moves_on_is_not_asked_for_ever() {
        let server = apple_server(|_| {
            json_reply(&json!({
                "data": [{ "id": "l.x", "type": "library-albums", "attributes": { "name": "Made-up album" } }],
                "next": "/v1/me/library/albums?offset=1",
            }))
        });
        let am = apple_music(&server).await;
        let ended = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            am.each_library_album(|_, _| {}),
        )
        .await;
        let asked = server.requests_to("/v1/me/library/albums").len();
        assert!(ended.is_ok(), "still asking after {asked} requests");
        assert!(asked <= 3, "{asked} requests for a list of one page");
    }
