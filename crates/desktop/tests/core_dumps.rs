//! S1 (stage 1 audit), ADR-0045: Opium leaves no core dump unless asked. Its own binary, as the
//! test lowers the process's limit for good and runs a child of itself.

use std::process::Command;

/// The process's core size limit, soft and hard.
fn core_limit() -> (u64, u64) {
    let mut limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    // SAFETY: `getrlimit` writes the struct it is given, which lives for the call.
    assert_eq!(unsafe { libc::getrlimit(libc::RLIMIT_CORE, &mut limit) }, 0);
    (limit.rlim_cur, limit.rlim_max)
}

/// Run as a child: say the limit before and after the call, with the environment it was given.
#[test]
fn child_reports_the_core_limit() {
    if std::env::var_os("OPIUM_TEST_CORE_CHILD").is_none() {
        return;
    }
    let before = core_limit();
    desktop::exit::keep_core_dumps_off();
    let after = core_limit();
    println!("limits: {before:?} -> {after:?}");
}

fn child(core_dumps: Option<&str>) -> String {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "child_reports_the_core_limit", "--nocapture"])
        .env("OPIUM_TEST_CORE_CHILD", "1")
        .env_remove("OPIUM_CORE_DUMPS");
    if let Some(value) = core_dumps {
        command.env("OPIUM_CORE_DUMPS", value);
    }
    let out = command.output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    stdout
        .lines()
        .find(|l| l.starts_with("limits: "))
        .unwrap_or_else(|| panic!("no limits in {stdout:?}"))
        .to_owned()
}

#[test]
fn the_core_size_limit_is_zero_soft_and_hard() {
    let line = child(None);
    assert!(line.ends_with("-> (0, 0)"), "{line}");
}

#[test]
fn opium_core_dumps_1_leaves_the_limit_as_it_was() {
    let line = child(Some("1"));
    let (before, after) = line["limits: ".len()..].split_once(" -> ").unwrap();
    assert_eq!(before, after, "{line}");
}

#[test]
fn another_value_turns_them_off_all_the_same() {
    let line = child(Some("yes"));
    assert!(line.ends_with("-> (0, 0)"), "{line}");
}
