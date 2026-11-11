//! ADR-0043: one Opium per profile, held by a lock (`desktop::instance`).
//!
//! The other processes are this test binary started again as children, each
//! told by a variable what to be. Every lock is taken in a folder of the
//! test's own under `std::env::temp_dir()`; a child that calls
//! `instance::take()` is given a config folder of its own, or none, never
//! the user's.

use std::io::{BufRead, BufReader, Read};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::{Mutex, MutexGuard};

use desktop::instance::{self, LOCK_FILE, Refused};

/// In a child: take the lock in the folder this names, and hold it until
/// stdin closes.
const HOLDER: &str = "OPIUM_TEST_LOCK_HOLDER";
/// In a child: hold nothing, and wait until stdin closes.
const IDLE: &str = "OPIUM_TEST_LOCK_IDLE";
/// In a child: take the profile's lock (`instance::take`), and say how it went.
const TAKER: &str = "OPIUM_TEST_LOCK_TAKER";
/// In a child: take the profile's lock, then sweep as `main` does
/// (`backend::sweep_partials`), and say how it went.
const SWEEPER: &str = "OPIUM_TEST_LOCK_SWEEPER";

/// Held by each test while it runs. A child started by one test holds a
/// copy of every file this process has open until it starts its program,
/// another test's lock among them, which would then stay held a moment
/// after that test let it go.
fn one_at_a_time() -> MutexGuard<'static, ()> {
    static ONE: Mutex<()> = Mutex::new(());
    ONE.lock().unwrap_or_else(|e| e.into_inner())
}

/// A folder of the test's own, gone after.
struct Dir(PathBuf);

impl Dir {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("opium-lock-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o700));
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// What this process is, as a child; whether it is one.
fn be_a_child() -> bool {
    if let Some(dir) = std::env::var_os(HOLDER) {
        let held = instance::take_in(Path::new(&dir));
        println!("{}", if held.is_ok() { "held" } else { "refused" });
        let _ = std::io::stdin().lock().read_line(&mut String::new());
        return true;
    }
    if std::env::var_os(IDLE).is_some() {
        println!("idle");
        let _ = std::io::stdin().lock().read_line(&mut String::new());
        return true;
    }
    if std::env::var_os(TAKER).is_some() {
        match instance::take() {
            Ok(_) => println!("taken"),
            Err(refused) => println!("refused: {refused:?}"),
        }
        return true;
    }
    if std::env::var_os(SWEEPER).is_some() {
        match instance::take() {
            Ok(held) => {
                desktop::backend::sweep_partials(&held);
                println!("swept");
            }
            Err(refused) => println!("refused: {refused:?}"),
        }
        return true;
    }
    false
}

/// This test started again as a child, told what to be by `env`.
fn child(test: &str, env: &[(&str, &std::ffi::OsStr)]) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", test, "--test-threads=1", "--nocapture"])
        .envs(env.iter().copied())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());
    command
}

/// Whether the child said `word`: at the end of a line, as the test
/// harness prints the test's name first on the same line.
fn said(line: &str, word: &str) -> bool {
    line.trim_end().ends_with(word)
}

/// Waits for the child to say `word`.
fn wait_for(stdout: &mut BufReader<ChildStdout>, word: &str) {
    let mut line = String::new();
    loop {
        line.clear();
        let read = stdout.read_line(&mut line).unwrap();
        assert!(read > 0, "the child ended before saying {word}");
        if said(&line, word) {
            return;
        }
    }
}

/// A child holding the lock in `dir`, once it says it holds it.
fn holder(test: &str, dir: &Path) -> (Child, BufReader<ChildStdout>) {
    let mut holder = child(test, &[(HOLDER, dir.as_os_str())]).spawn().unwrap();
    let mut stdout = BufReader::new(holder.stdout.take().unwrap());
    wait_for(&mut stdout, "held");
    (holder, stdout)
}

fn is_running(taken: &Result<instance::Instance, Refused>) -> bool {
    matches!(taken, Err(Refused::Running))
}

#[test]
fn a_second_take_fails_while_another_process_holds_the_lock() {
    if be_a_child() {
        return;
    }
    let _turn = one_at_a_time();
    let dir = Dir::new("ended");
    let (mut holder, _stdout) = holder(
        "a_second_take_fails_while_another_process_holds_the_lock",
        &dir.0,
    );
    let while_held = instance::take_in(&dir.0);
    // The holder ends of itself.
    drop(holder.stdin.take());
    assert!(holder.wait().unwrap().success());
    let once_ended = instance::take_in(&dir.0);
    assert!(
        is_running(&while_held),
        "while another held it: {while_held:?}"
    );
    assert!(once_ended.is_ok(), "once it ended: {once_ended:?}");
}

#[test]
fn the_lock_of_a_killed_holder_is_free() {
    if be_a_child() {
        return;
    }
    let _turn = one_at_a_time();
    let dir = Dir::new("killed");
    let (mut holder, _stdout) = holder("the_lock_of_a_killed_holder_is_free", &dir.0);
    let while_held = instance::take_in(&dir.0);
    // As a crash would end it, saving nothing and closing nothing itself.
    holder.kill().unwrap();
    let _ = holder.wait();
    let once_killed = instance::take_in(&dir.0);
    assert!(
        is_running(&while_held),
        "while another held it: {while_held:?}"
    );
    assert!(once_killed.is_ok(), "once it was killed: {once_killed:?}");
}

#[test]
fn a_second_take_in_the_same_process_fails() {
    let _turn = one_at_a_time();
    let dir = Dir::new("same");
    let first = instance::take_in(&dir.0).unwrap();
    assert!(is_running(&instance::take_in(&dir.0)));
    drop(first);
    assert!(
        instance::take_in(&dir.0).is_ok(),
        "free once the first is dropped"
    );
}

/// A program Opium starts, such as the sign-in browser, must not hold the
/// lock on after Opium has ended.
#[test]
fn a_program_started_while_the_lock_is_held_keeps_none_of_it() {
    if be_a_child() {
        return;
    }
    let _turn = one_at_a_time();
    let dir = Dir::new("inherited");
    let lock = instance::take_in(&dir.0).unwrap();
    let mut started = child(
        "a_program_started_while_the_lock_is_held_keeps_none_of_it",
        &[(IDLE, "1".as_ref())],
    )
    .spawn()
    .unwrap();
    let mut stdout = BufReader::new(started.stdout.take().unwrap());
    wait_for(&mut stdout, "idle");
    drop(lock);
    let while_it_runs = instance::take_in(&dir.0);
    drop(started.stdin.take());
    let _ = started.wait();
    assert!(
        while_it_runs.is_ok(),
        "the program started kept the lock: {while_it_runs:?}"
    );
}

/// A lock file left read-only, by a profile restored from a backup say:
/// `flock` needs no write access, so it must not keep Opium from starting.
#[test]
fn a_lock_file_left_read_only_still_holds_the_lock() {
    let _turn = one_at_a_time();
    let dir = Dir::new("read-only");
    let file = dir.0.join(LOCK_FILE);
    std::fs::write(&file, "").unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o400)).unwrap();
    let first = instance::take_in(&dir.0);
    let second = instance::take_in(&dir.0);
    assert!(first.is_ok(), "{first:?}");
    assert!(is_running(&second), "{second:?}");
}

#[test]
fn a_folder_that_cannot_be_written_refuses_the_lock() {
    let _turn = one_at_a_time();
    let dir = Dir::new("unwritable");
    std::fs::set_permissions(&dir.0, std::fs::Permissions::from_mode(0o500)).unwrap();
    assert!(
        std::fs::File::create(dir.0.join("probe")).is_err(),
        "run as a user that permissions apply to, not root"
    );
    let config = dir.0.join("opium");
    let taken = instance::take_in(&config);
    match &taken {
        Err(Refused::Failed { error, .. }) => {
            assert_eq!(
                error.kind(),
                std::io::ErrorKind::PermissionDenied,
                "{error}"
            );
        }
        other => panic!("taken in a folder that can't be written: {other:?}"),
    }
    assert!(!config.exists());
}

#[test]
fn a_file_where_the_folder_should_be_refuses_the_lock() {
    let _turn = one_at_a_time();
    let dir = Dir::new("file");
    let config = dir.0.join("opium");
    std::fs::write(&config, "not a folder").unwrap();
    let taken = instance::take_in(&config);
    assert!(
        matches!(taken, Err(Refused::Failed { .. })),
        "taken where a file is: {taken:?}"
    );
    assert_eq!(std::fs::read_to_string(&config).unwrap(), "not a folder");
}

/// `instance::take`, in a child whose environment is `env` with `HOME` and
/// the XDG variables removed first: what it said.
fn take_with(test: &str, env: &[(&str, &std::ffi::OsStr)]) -> String {
    run_with(test, TAKER, env)
}

/// The child `role` names, in an environment that is `env` with `HOME` and
/// the XDG variables removed first: what it said.
fn run_with(test: &str, role: &str, env: &[(&str, &std::ffi::OsStr)]) -> String {
    let mut command = child(test, &[]);
    for variable in ["HOME", "XDG_CONFIG_HOME", "XDG_CACHE_HOME", "XDG_DATA_HOME"] {
        command.env_remove(variable);
    }
    let mut taker = command
        .envs(env.iter().copied())
        .env(role, "1")
        .spawn()
        .unwrap();
    drop(taker.stdin.take());
    let mut said = String::new();
    taker
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut said)
        .unwrap();
    assert!(taker.wait().unwrap().success(), "{said}");
    said
}

#[test]
fn the_profiles_lock_is_in_its_config_folder() {
    if be_a_child() {
        return;
    }
    let _turn = one_at_a_time();
    let dir = Dir::new("profile");
    let said = take_with(
        "the_profiles_lock_is_in_its_config_folder",
        &[("XDG_CONFIG_HOME", dir.0.as_os_str())],
    );
    assert!(said.lines().any(|line| self::said(line, "taken")), "{said}");
    assert!(dir.0.join("opium").join(LOCK_FILE).is_file());
}

#[test]
fn without_a_config_folder_there_is_no_lock_to_take() {
    if be_a_child() {
        return;
    }
    let _turn = one_at_a_time();
    for env in [
        &[][..],
        &[("XDG_CONFIG_HOME", "".as_ref()), ("HOME", "".as_ref())][..],
        &[
            ("XDG_CONFIG_HOME", "config".as_ref()),
            ("HOME", "home".as_ref()),
        ][..],
    ] {
        let said = take_with("without_a_config_folder_there_is_no_lock_to_take", env);
        assert!(
            said.lines()
                .any(|line| self::said(line, "refused: NoConfigFolder")),
            "{env:?}: {said}"
        );
    }
}

/// Swept once the lock is taken, as `main` does, the partial files a crash
/// left go from every folder written whole; the downloads', a sign-in
/// browser's profile and every other file stay. Taking the lock alone
/// sweeps nothing.
#[test]
fn the_partial_files_a_crash_left_go_once_the_lock_is_taken() {
    if be_a_child() {
        return;
    }
    let _turn = one_at_a_time();
    let dir = Dir::new("sweep");
    let (config, cache, data) = (
        dir.0.join("config/opium"),
        dir.0.join("cache/opium"),
        dir.0.join("data/opium"),
    );
    let left = [
        config.join("settings.part"),
        config.join(".0123456789abcdef.part"),
        config.join("shared.json.part"),
        config.join("apple-music/media-user-token.part"),
        config.join("soundcloud/tokens.part"),
        config.join("navidrome/credentials.part"),
        config.join("covers/p.1-00ff00ff00ff00ff.part"),
        cache.join("library/playlist-p.AbC.json.part.3"),
        cache.join("covers/0123456789abcdef.part"),
        cache.join("motion/0123456789abcdef-600.part"),
        cache.join("local/index.part.42.0"),
        cache.join("local/covers/0123456789abcdef.part"),
    ];
    let kept = [
        config.join("settings.toml"),
        config.join("apple-music/browser-profile/Default.part"),
        data.join("downloads/0123456789abcdef.part"),
    ];
    for file in left.iter().chain(&kept) {
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, "x").unwrap();
    }
    let test = "the_partial_files_a_crash_left_go_once_the_lock_is_taken";
    let (config_home, cache_home, data_home) = (
        dir.0.join("config"),
        dir.0.join("cache"),
        dir.0.join("data"),
    );
    let env = [
        ("XDG_CONFIG_HOME", config_home.as_os_str()),
        ("XDG_CACHE_HOME", cache_home.as_os_str()),
        ("XDG_DATA_HOME", data_home.as_os_str()),
    ];
    let said = take_with(test, &env);
    assert!(said.lines().any(|line| self::said(line, "taken")), "{said}");
    assert!(
        left.iter().all(|file| file.exists()),
        "taking the lock swept"
    );
    let said = run_with(test, SWEEPER, &env);
    assert!(said.lines().any(|line| self::said(line, "swept")), "{said}");
    let still: Vec<&PathBuf> = left.iter().filter(|file| file.exists()).collect();
    assert!(still.is_empty(), "left: {still:?}");
    let gone: Vec<&PathBuf> = kept.iter().filter(|file| !file.exists()).collect();
    assert!(gone.is_empty(), "swept: {gone:?}");
}
