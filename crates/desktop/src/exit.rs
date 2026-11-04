//! Leaving the app. Once what it keeps is on disk, nothing may hold it
//! open: its teardown (the window's, the player's, the other devices', the
//! CDM's at exit) has now and then been seen never to end, the window gone
//! and the process left behind, to be killed by hand. A watch then ends it,
//! after noting where it stood, so that the cause can be found.

use std::io::Write;
use std::sync::{Mutex, Once};
use std::time::Duration;

/// How long the teardown is given once all is kept: the player waits half a
/// second for its engine, the other devices a second to say goodbye.
const GRACE: Duration = Duration::from_secs(5);

static STAGE: Mutex<&str> = Mutex::new("");
static WATCH: Once = Once::new();

/// What is kept is on disk: from now on the app ends within [`GRACE`],
/// torn down or not.
pub fn leaving() {
    reached("tearing down");
    WATCH.call_once(|| {
        let _ = std::thread::Builder::new()
            .name("exit-watch".into())
            .spawn(|| {
                std::thread::sleep(GRACE);
                report();
                // Not `exit`: whatever held the teardown may be what it
                // would run again.
                unsafe { libc::_exit(0) }
            });
    });
}

/// How far the teardown has come, for the note if it hangs.
pub fn reached(stage: &'static str) {
    *STAGE.lock().unwrap_or_else(|e| e.into_inner()) = stage;
}

/// Where the teardown stood, and the threads still there with what each
/// waits on, added to `exit.log` in the cache and said on stderr.
fn report() {
    let stage = *STAGE.lock().unwrap_or_else(|e| e.into_inner());
    let mut note = format!(
        "{} s since 1970: still not ended {} s after leaving, at: {stage}\n",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs()),
        GRACE.as_secs()
    );
    if let Ok(tasks) = std::fs::read_dir("/proc/self/task") {
        for task in tasks.flatten() {
            let read = |name: &str| {
                std::fs::read_to_string(task.path().join(name))
                    .map(|s| s.trim().to_owned())
                    .unwrap_or_default()
            };
            note += &format!(
                "  {} {:?}: waits in {}, syscall {}\n",
                task.file_name().to_string_lossy(),
                read("comm"),
                read("wchan"),
                read("syscall").split(' ').next().unwrap_or(""),
            );
        }
    }
    eprint!("opium: {note}");
    if let Some(dir) = crate::covers::cache_dir("") {
        let _ = std::fs::create_dir_all(&dir);
        if let Ok(mut log) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("exit.log"))
        {
            let _ = log.write_all(note.as_bytes());
        }
    }
}
