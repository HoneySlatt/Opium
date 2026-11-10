    // Two parts, for two crates.

    // -----------------------------------------------------------------------------------------
    // Part 1. Belongs in: crates/soundcloud/src/lib.rs, module `tests` (lib.rs:1019).

    /// The bot guard's refusal is told by its header, or, where the header can't be read, by its
    /// body naming the check; another refusal is not the guard's. `call` gives up on the guard's
    /// at once, without reading a new client id nor trying again (ADR-0012: no refused route
    /// retried). No test reached `guarded` before.
    ///
    /// Kills `lib.rs:64:5 replace guarded -> bool with false` (a clean miss), `… with true`, and
    /// `guarded`'s operator mutants (`==` with `!=`, `&&` with `||`, `||` with `&&`).
    #[test]
    fn the_bot_guards_refusal_is_told_by_its_header_or_its_body() {
        assert!(guarded(403, true, ""));
        assert!(guarded(
            403,
            false,
            r#"<script src="https://ct.captcha-delivery.com/c.js"></script>"#
        ));
        assert!(!guarded(403, false, "forbidden"));
        assert!(!guarded(401, true, ""));
        assert!(!guarded(429, false, "captcha-delivery.com"));
    }

    // -----------------------------------------------------------------------------------------
    // Part 2. Belongs in: crates/navidrome/src/lib.rs, module `year` (lib.rs:64), in place of
    // `this_year_is_read_off_the_clock` or beside it.

    /// This year by the clock is the calendar's: opium-core's reading of a date
    /// (`opium_core::recent::rfc3339`), another algorithm, says the same year. The test before
    /// asks only for 2026 or later, which any wrong year after that passes, and fails on a
    /// machine whose clock is behind. This one still reads the clock, twice: it could be wrong a
    /// few microseconds a year, at the turn of the year in UTC.
    ///
    /// Kills 21 of the 34 `current_year` mutants the audit's run recorded missed, 6 of its 8
    /// clean misses among them (run in a copy, on 2026-11-10: proof `.delegation/m18-tests/21.out`).
    /// The 13 others (57:33, 57:48, 58:19 `/`, 58:32, 58:38, 58:42, 58:48 `%`, 59:23 `-`) change
    /// the year only on some days: January and February, the turn of a century. A test of today
    /// cannot reach them; that needs `current_year` split into a function of the day, tested on
    /// known dates (1970-01-01, 2000-02-29, 2100-03-01, a 31 December and a 1 January).
    #[test]
    fn this_year_is_the_calendars() {
        let seconds = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let year: i64 = opium_core::recent::rfc3339(seconds)[..4].parse().unwrap();
        assert_eq!(super::current_year(), year);
    }
