//! Stage 1 audit, L2: two instances of Opium never share its MPRIS name, nor a profile.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use desktop::instance::{Refused, take_in};
use desktop::mpris::{Mpris, raise_running};

const NAME: &str = "org.mpris.MediaPlayer2.opium";
/// Set in the child process that plays the Opium already running.
const FIRST: &str = "OPIUM_TEST_FIRST_INSTANCE";
/// Set once this test runs on a session bus of its own.
const OWN_BUS: &str = "OPIUM_TEST_OWN_BUS";
/// What the first instance prints once `Mpris::start` has kept its name,
/// followed by `yes` or `no`.
const KEPT: &str = "name kept: ";

/// Whether this process runs on a session bus of its own. If it doesn't,
/// runs `test` again under `dbus-run-session`, which starts one for it
/// alone, and checks that it passed there: the bus of the session the tests
/// were started in is never used, as the user's Opium may be on it.
fn on_a_bus_of_its_own(test: &str) -> bool {
    if std::env::var_os(OWN_BUS).is_some() {
        return true;
    }
    let status = Command::new("dbus-run-session")
        .arg(format!("--config-file={}", bus_config().display()))
        .arg("--")
        .arg(std::env::current_exe().expect("the test binary"))
        .args(["--exact", test, "--test-threads=1", "--nocapture"])
        .env(OWN_BUS, "1")
        .status()
        .expect("dbus-run-session, which gives these tests a session bus of their own");
    assert!(
        status.success(),
        "{test} failed on a session bus of its own"
    );
    false
}

/// A session bus's configuration of the test's own, written to its folder
/// under the temporary one: `dbus-run-session`'s default is a file of the
/// system's (`/etc/dbus-1/session.conf`), which not every machine has.
fn bus_config() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("opium-test-bus-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("the test's folder");
    let path = dir.join("session.conf");
    std::fs::write(
        &path,
        format!(
            r#"<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-BUS Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <type>session</type>
  <listen>unix:dir={}</listen>
  <policy context="default">
    <allow send_destination="*" eavesdrop="true"/>
    <allow eavesdrop="true"/>
    <allow own="*"/>
  </policy>
</busconfig>
"#,
            dir.display()
        ),
    )
    .expect("the bus's configuration");
    path
}

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

/// The Opium already running, and its profile: a config folder of its own.
struct First {
    child: Child,
    stdout: BufReader<ChildStdout>,
    profile: PathBuf,
}

impl First {
    /// Its config folder, where its lock is.
    fn config(&self) -> PathBuf {
        self.profile.join("opium")
    }

    /// Waits until it says its name is kept from later requests, so that no
    /// test runs in the moment between its two requests for the name.
    fn wait_until_kept(&mut self) {
        let mut line = String::new();
        loop {
            line.clear();
            let read = self.stdout.read_line(&mut line).expect("its output");
            assert!(
                read > 0,
                "the first instance ended before saying whether its name is kept"
            );
            // The harness prints the test's name first on the same line.
            if let Some((_, said)) = line.trim_end().rsplit_once(KEPT) {
                assert_eq!(said, "yes", "the first instance's name was not kept");
                return;
            }
        }
    }
}

/// Starts this test again in a child that takes its profile's lock first, as
/// `main` does, then, once `delay` has passed, the name as `App::new` does,
/// and keeps both until its stdin closes.
fn first_instance(test: &str, delay: Duration) -> First {
    let profile = std::env::temp_dir().join(format!(
        "opium-single-instance-{test}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&profile);
    let mut child = Command::new(std::env::current_exe().expect("the test binary"))
        .args(["--exact", test, "--test-threads=1", "--nocapture"])
        .env(FIRST, delay.as_millis().to_string())
        .env("XDG_CONFIG_HOME", &profile)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("the first instance started");
    let stdout = BufReader::new(child.stdout.take().expect("its output"));
    First {
        child,
        stdout,
        profile,
    }
}

/// In the child: be the Opium already running. Returns whether this process
/// is the child.
fn be_first_instance() -> bool {
    let Some(delay) = std::env::var_os(FIRST) else {
        return false;
    };
    let delay: u64 = delay.to_str().and_then(|d| d.parse().ok()).unwrap_or(0);
    let _instance = desktop::instance::take().expect("the first instance's lock");
    std::thread::sleep(Duration::from_millis(delay));
    let mpris = Mpris::start(no_wake());
    let until = Instant::now() + Duration::from_secs(5);
    while !mpris.on_the_bus() && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(5));
    }
    println!("{KEPT}{}", if mpris.on_the_bus() { "yes" } else { "no" });
    let _ = std::io::stdin().lock().read_line(&mut String::new());
    true
}

fn quit(mut first: First) {
    drop(first.child.stdin.take());
    let _ = first.child.wait();
    let _ = std::fs::remove_dir_all(&first.profile);
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
    if !on_a_bus_of_its_own("a_second_instance_leaves_the_name_to_the_first") {
        return;
    }
    let mut first = first_instance(
        "a_second_instance_leaves_the_name_to_the_first",
        Duration::ZERO,
    );
    let first_pid = first.child.id();
    wait_until_held(true, "the first instance never took the name");
    first.wait_until_kept();
    // The second instance's App::new: one on another profile, whose lock
    // (see the test below) didn't stop it.
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

/// The race: `main` used to look for a running Opium (`raise_running`)
/// before the window existed, and the instance takes the name only at the
/// end of `App::new`, on a thread of its own, about 170 ms later on the
/// owner's machine (docs/measurements.md, warm start-up). A launch in
/// between was told that none runs, and opened a second window. `main` now
/// takes the profile's lock first, which the first holds from its start.
/// The child here has already passed its own check and is "building its
/// window" for 300 ms.
#[test]
fn a_launch_while_another_starts_finds_it() {
    if be_first_instance() {
        return;
    }
    if !on_a_bus_of_its_own("a_launch_while_another_starts_finds_it") {
        return;
    }
    assert!(!raise_running(), "the bus already has an Opium");
    let first = first_instance(
        "a_launch_while_another_starts_finds_it",
        Duration::from_millis(300),
    );
    std::thread::sleep(Duration::from_millis(100));
    // What `main` asks first, on the same profile: whether its lock is held.
    let found = matches!(take_in(&first.config()), Err(Refused::Running));
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
    if !on_a_bus_of_its_own("a_launch_while_one_runs_raises_it") {
        return;
    }
    let first = first_instance("a_launch_while_one_runs_raises_it", Duration::ZERO);
    wait_until_held(true, "the first instance never took the name");
    let found = raise_running();
    let opened = desktop::mpris::open_in_running(&[std::path::PathBuf::from("/music/a.flac")]);
    quit(first);
    assert!(
        found && opened,
        "raised: {found}, files handed over: {opened}"
    );
}
