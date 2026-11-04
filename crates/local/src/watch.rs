//! The folders watched: a file added, moved, changed or deleted reads them
//! again, once things have settled.

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use notify::{EventKind, RecursiveMode, Watcher};

/// How long the folders stay quiet before they are read again: a copy of an
/// album is one reading, not one per file.
const SETTLED: Duration = Duration::from_millis(1500);

/// Watching while kept; dropped, it stops.
pub struct Watching {
    _watcher: notify::RecommendedWatcher,
}

/// Watch `folders`, calling `changed` once they settle after any change.
pub fn watch(folders: &[PathBuf], changed: impl Fn() + Send + 'static) -> notify::Result<Watching> {
    let (tx, rx) = mpsc::channel::<()>();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if let Ok(event) = event
            && !matches!(event.kind, EventKind::Access(_))
        {
            let _ = tx.send(());
        }
    })?;
    for folder in folders {
        watcher.watch(folder, RecursiveMode::Recursive)?;
    }
    std::thread::spawn(move || {
        // Ends with the watcher, its sender gone.
        while rx.recv().is_ok() {
            loop {
                match rx.recv_timeout(SETTLED) {
                    Ok(()) => {}
                    Err(mpsc::RecvTimeoutError::Timeout) => break,
                    Err(mpsc::RecvTimeoutError::Disconnected) => return,
                }
            }
            changed();
        }
    });
    Ok(Watching { _watcher: watcher })
}
