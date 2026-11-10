//! Belongs in `crates/desktop/tests/single_instance.rs` (an integration test of
//! `desktop`, ADR-0018). It needs a session bus of its own, and its tests run
//! one at a time:
//!
//! ```sh
//! unshare --user --map-current-user --net \
//!     dbus-run-session -- cargo test -p desktop --test single_instance -- --test-threads=1 --nocapture
//! ```
//!
//! The Opium already running is played by this same test binary started again
//! as a child process, which holds the name through `Mpris::start` until its
//! stdin closes. Nothing here opens a window, plays sound or reads a profile.

use std::io::BufRead;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use desktop::mpris::{Mpris, raise_running};

const NAME: &str = "org.mpris.MediaPlayer2.opium";
/// Set in the child process that plays the Opium already running.
const FIRST: &str = "OPIUM_TEST_FIRST_INSTANCE";

fn no_wake() -> desktop::Wake {
    Arc::new(|| {})
}

/// Whether any connection on the session bus holds Opium's MPRIS name.
fn name_held() -> bool {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime");
    runtime.block_on(async {
        let bus = mpris_server::zbus::Connection::session()
            .await
            .expect("a session bus: run under dbus-run-session");
        let reply = bus
            .call_method(
                Some("org.freedesktop.DBus"),
                "/org/freedesktop/DBus",
                Some("org.freedesktop.DBus"),
                "NameHasOwner",
                &(NAME,),
            )
            .await
            .expect("NameHasOwner answered");
        reply.body().deserialize::<bool>().expect("a boolean")
    })
}

fn wait_until_held(held: bool, what: &str) {
    let until = Instant::now() + Duration::from_secs(5);
    while name_held() != held {
        assert!(Instant::now() < until, "{what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Starts this test again in a child that, once `delay` has passed, takes
/// the name as `App::new` does and keeps it until its stdin closes.
fn first_instance(test: &str, delay: Duration) -> Child {
    Command::new(std::env::current_exe().expect("the test binary"))
        .args(["--exact", test, "--test-threads=1", "--nocapture"])
        .env(FIRST, delay.as_millis().to_string())
        .stdin(Stdio::piped())
        .spawn()
        .expect("the first instance started")
}

/// In the child: be the Opium already running. Returns whether this process
/// is the child.
fn be_first_instance() -> bool {
    let Some(delay) = std::env::var_os(FIRST) else {
        return false;
    };
    let delay: u64 = delay.to_str().and_then(|d| d.parse().ok()).unwrap_or(0);
    std::thread::sleep(Duration::from_millis(delay));
    let _mpris = Mpris::start(no_wake());
    let _ = std::io::stdin().lock().read_line(&mut String::new());
    true
}

fn quit(mut first: Child) {
    drop(first.stdin.take());
    let _ = first.wait();
}

/// The process holding Opium's name, by its pid.
fn owner_pid() -> Option<u32> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime");
    runtime.block_on(async {
        let bus = mpris_server::zbus::Connection::session().await.ok()?;
        let dbus = |method: &'static str, arg: String| {
            let bus = bus.clone();
            async move {
                bus.call_method(
                    Some("org.freedesktop.DBus"),
                    "/org/freedesktop/DBus",
                    Some("org.freedesktop.DBus"),
                    method,
                    &(arg,),
                )
                .await
                .ok()
            }
        };
        let owner = dbus("GetNameOwner", NAME.to_owned())
            .await?
            .body()
            .deserialize::<String>()
            .ok()?;
        dbus("GetConnectionUnixProcessID", owner)
            .await?
            .body()
            .deserialize::<u32>()
            .ok()
    })
}

/// The debt's observation (`docs/debt.md`, "Two instances can run at
/// once"): a second instance that reaches `Mpris::start` takes the name from
/// the one running, which is left without it and not even queued for it; once
/// the newer one quits, nobody holds the name, though the older still runs.
#[test]
fn a_second_instance_leaves_the_name_to_the_first() {
    if be_first_instance() {
        return;
    }
    let first = first_instance(
        "a_second_instance_leaves_the_name_to_the_first",
        Duration::ZERO,
    );
    let first_pid = first.id();
    wait_until_held(true, "the first instance never took the name");
    // The second instance's App::new, its check in main() having found none
    // (see the test below) or failed.
    let second = Mpris::start(no_wake());
    std::thread::sleep(Duration::from_millis(500));
    let owner_with_both = owner_pid();
    // The newer one quits; the first runs on.
    drop(second);
    std::thread::sleep(Duration::from_millis(500));
    let held_after = name_held();
    quit(first);
    assert_eq!(
        (owner_with_both, held_after),
        (Some(first_pid), true),
        "(owner while both ran, name held once the newer quit): the second took the \
         name from the first (pid {first_pid}), and once it quit nobody held it, though \
         the first still ran"
    );
}

/// The race: `main` looks for a running Opium (`raise_running`) before the
/// window exists, and the instance takes the name only at the end of
/// `App::new`, on a thread of its own, about 170 ms later on the owner's
/// machine (docs/measurements.md, warm start-up). A launch in between is
/// told that none runs, and opens a second window. The child here has
/// already passed its own check and is "building its window" for 300 ms.
#[test]
fn a_launch_while_another_starts_finds_it() {
    if be_first_instance() {
        return;
    }
    assert!(!raise_running(), "the bus already has an Opium");
    let first = first_instance(
        "a_launch_while_another_starts_finds_it",
        Duration::from_millis(300),
    );
    std::thread::sleep(Duration::from_millis(100));
    let found = raise_running();
    quit(first);
    assert!(
        found,
        "a second launch 100 ms after the first was told no Opium runs, so main() \
         opens a second window"
    );
}

/// The check itself, once the name is held: it finds the Opium running and
/// raises it, so that `main` exits. Passes today; kept as the guard the fix
/// must not break.
#[test]
fn a_launch_while_one_runs_raises_it() {
    if be_first_instance() {
        return;
    }
    let first = first_instance("a_launch_while_one_runs_raises_it", Duration::ZERO);
    wait_until_held(true, "the first instance never took the name");
    let found = raise_running();
    let opened = desktop::mpris::open_in_running(&[std::path::PathBuf::from("/music/a.flac")]);
    quit(first);
    assert!(found && opened, "raised: {found}, files handed over: {opened}");
}
