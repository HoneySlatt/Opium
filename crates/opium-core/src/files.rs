//! Files kept safe on the disk: written whole or not at all, and put aside
//! rather than lost when they don't read.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Who may read a file that [`write_whole`] writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Whoever the process's umask lets: for files that hold no secret.
    Default,
    /// The user alone (`0600` on Unix), from the file's creation on: for
    /// secrets, and the files kept with them.
    Private,
}

/// How many names [`write_whole`] tries for its partial file: each is
/// random, so a second is needed only if another file has the first.
const PARTIAL_NAMES_TRIED: usize = 8;

/// Write `bytes` to `path` whole or not at all: into a new partial file in
/// the same folder, of a name no other writer uses, put on disk, renamed
/// over `path`, and the folder then put on disk too, so that the rename
/// itself lasts.
///
/// Whatever happens (a crash, a power cut, two writers at once), a reader
/// finds `path` as it was or as one writer wrote it whole, never a mix; of
/// two writers at once, the last to rename wins.
///
/// The partial file is `.<16 hex digits>.part`: short, so that a file of
/// the longest name a folder takes is still written; hidden; ending in
/// `.part` for a folder's own sweep. It is made with `create_new`, so a
/// file found under that name (left by a crash, or put there by someone
/// else) is never written into, and `mode` holds from its creation. A crash
/// before the rename leaves it behind, for [`sweep_partials`] to remove at
/// the next start.
///
/// `path` is replaced, not written into: it takes `mode`, not the old
/// file's permissions; a link there is replaced by the file; another hard
/// link to the old file keeps the old bytes. The folder must exist. Its
/// flush, last, is not an error when it fails: `path` is whole either way,
/// though a power cut may then bring the old file back. The first such
/// failure in the process is said on standard error, naming the folder.
///
/// # Errors
///
/// `path` names no file (it ends in `..`, or is a root), the folder can't
/// take a new file, the disk is full, or the rename fails (`path` is a
/// folder, say). `path` is then as it was, and the partial file removed.
pub fn write_whole(path: &Path, bytes: &[u8], mode: Mode) -> io::Result<()> {
    write_whole_with(path, bytes, mode, None)
}

/// [`write_whole`], running `before_rename` once the partial file is on
/// disk, just before the rename: for a test to stop a writer there.
fn write_whole_with(
    path: &Path,
    bytes: &[u8],
    mode: Mode,
    before_rename: Option<&dyn Fn()>,
) -> io::Result<()> {
    if path.file_name().is_none() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "the path names no file",
        ));
    }
    let (partial, mut file) = new_partial(path, mode)?;
    let written = file
        .write_all(bytes)
        .and_then(|()| file.sync_all())
        .and_then(|()| {
            if let Some(before_rename) = before_rename {
                before_rename();
            }
            std::fs::rename(&partial, path)
        });
    drop(file);
    if let Err(e) = written {
        let _ = std::fs::remove_file(&partial);
        return Err(e);
    }
    let folder = folder_of(path);
    if let Err(e) = File::open(folder).and_then(|opened| opened.sync_all()) {
        // Said once: a file system that can't flush a folder fails every time.
        static SAID: std::sync::Once = std::sync::Once::new();
        SAID.call_once(|| {
            eprintln!(
                "opium: the folder {} couldn't be put on disk ({e}): what is written there is \
                 whole, but a power cut may bring back the files before (said once)",
                folder.display()
            );
        });
    }
    Ok(())
}

/// A new file beside `path`, under a random name no file had, open to
/// write, readable as `mode` says.
fn new_partial(path: &Path, mode: Mode) -> io::Result<(PathBuf, File)> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    if mode == Mode::Private {
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    }
    let mut tried = 1;
    loop {
        let partial = path.with_file_name(format!(".{:016x}.part", rand::random::<u64>()));
        match options.open(&partial) {
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists && tried < PARTIAL_NAMES_TRIED => {
                tried += 1;
            }
            opened => return opened.map(|file| (partial, file)),
        }
    }
}

/// The folder `path` is in: `.` for a name alone.
fn folder_of(path: &Path) -> &Path {
    match path.parent() {
        Some(folder) if !folder.as_os_str().is_empty() => folder,
        _ => Path::new("."),
    }
}

/// Remove, in `dir` and not below it, the partial files a crash left
/// behind, before the rename that would have put them in place; return how
/// many went.
///
/// Those are [`write_whole`]'s, `.<16 hex digits>.part`, and those older
/// builds wrote aside under a fixed name beside each file they kept whole:
/// `<name>.part` (`settings.part`, `connect.json.part`,
/// `media-user-token.part`, a cover's `<hash>.part`…), the library cache's
/// `<name>.part.<count>` and the files' index's `index.part.<pid>.<count>`.
/// Only regular files go: a folder or a link of such a name stays, and so
/// does every other file.
///
/// Run it only where every file is the app's, while nothing writes there:
/// at start, before anything is written, with the instance alone. A
/// partial file being written would go, and its write fail. Never in a
/// folder the user chose, nor in one with a sweep of its own (the
/// downloads').
///
/// A folder that is missing or can't be listed is left as it is, and so is
/// a file that can't be removed: the next start tries again.
pub fn sweep_partials(dir: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut removed = 0;
    for entry in entries.flatten() {
        let partial = entry.file_name().to_str().is_some_and(is_partial)
            && entry.file_type().is_ok_and(|kind| kind.is_file());
        if partial && std::fs::remove_file(entry.path()).is_ok() {
            removed += 1;
        }
    }
    removed
}

/// Whether `name` is a partial file's, as [`sweep_partials`] says: a name
/// followed by `.part`, then by nothing or by numbers each after a dot.
fn is_partial(name: &str) -> bool {
    let Some((kept, after)) = name.rsplit_once(".part") else {
        return false;
    };
    let numbered = |after: &str| {
        after.strip_prefix('.').is_some_and(|numbers| {
            numbers
                .split('.')
                .all(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        })
    };
    !kept.is_empty() && (after.is_empty() || numbered(after))
}

/// Move `path` to `aside`, or, when that name is taken, to `aside` followed
/// by `.1`, `.2`… the first name free; return the name used.
///
/// It never replaces an existing file, so that a file put aside a second
/// time leaves the first copy where it was. Each name is first taken by
/// creating an empty file there with `create_new`, which fails on a name
/// that exists (a dangling link included); then `path` is renamed over that
/// empty file, its own. Linking `path` to the new name and then unlinking it
/// would never replace either, but Android refuses hard links to apps (AOSP's
/// `system/sepolicy`, `private/app_neverallows.te`: `neverallow
/// all_untrusted_apps file_type:file link;`), as do filesystems such as FAT;
/// and the standard library has no rename that refuses to replace.
///
/// A crash between the two steps leaves `path` where it was, beside an empty
/// file under the new name, which the next call passes over.
///
/// # Errors
///
/// `path` missing ([`io::ErrorKind::NotFound`]), a folder that can't be
/// written, `aside` on another filesystem than `path`, or any other error of
/// the creation or the rename. `path` is then left where it was, and the
/// empty file made for it removed.
pub fn put_aside(path: &Path, aside: &Path) -> io::Result<PathBuf> {
    let mut name = aside.to_owned();
    let mut taken = 0u64;
    while let Err(e) = OpenOptions::new().write(true).create_new(true).open(&name) {
        if e.kind() != io::ErrorKind::AlreadyExists {
            return Err(e);
        }
        taken += 1;
        let mut numbered = aside.as_os_str().to_owned();
        numbered.push(format!(".{taken}"));
        name = PathBuf::from(numbered);
    }
    match std::fs::rename(path, &name) {
        Ok(()) => Ok(name),
        Err(e) => {
            let _ = std::fs::remove_file(&name);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder of the test's own under the temporary folder, gone after.
    struct Dir(PathBuf);
    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn dir() -> Dir {
        let dir = std::env::temp_dir().join(format!("files-{:016x}", rand::random::<u64>()));
        std::fs::create_dir_all(&dir).unwrap();
        Dir(dir)
    }

    #[test]
    fn each_file_put_aside_takes_the_first_name_free() {
        let dir = dir();
        let path = dir.0.join("shared.json");
        let aside = dir.0.join("shared.json.bad");
        for (copy, name) in [
            ("first", "shared.json.bad"),
            ("second", "shared.json.bad.1"),
            ("third", "shared.json.bad.2"),
        ] {
            std::fs::write(&path, copy).unwrap();
            assert_eq!(put_aside(&path, &aside).unwrap(), dir.0.join(name));
            assert!(!path.exists());
        }
        // Every copy is still there, under its own name.
        for (copy, name) in [
            ("first", "shared.json.bad"),
            ("second", "shared.json.bad.1"),
            ("third", "shared.json.bad.2"),
        ] {
            assert_eq!(std::fs::read_to_string(dir.0.join(name)).unwrap(), copy);
        }
    }

    #[test]
    fn a_name_free_before_one_taken_is_used() {
        let dir = dir();
        let path = dir.0.join("manifest.json");
        let aside = dir.0.join("manifest.corrupt.json");
        std::fs::write(dir.0.join("manifest.corrupt.json.1"), "kept").unwrap();
        std::fs::write(&path, "damaged").unwrap();
        assert_eq!(put_aside(&path, &aside).unwrap(), aside);
        assert_eq!(
            std::fs::read_to_string(dir.0.join("manifest.corrupt.json.1")).unwrap(),
            "kept"
        );
    }

    #[test]
    fn a_missing_file_is_an_error_that_leaves_nothing_behind() {
        let dir = dir();
        let aside = dir.0.join("shared.json.bad");
        let e = put_aside(&dir.0.join("shared.json"), &aside).unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::NotFound);
        assert_eq!(std::fs::read_dir(&dir.0).unwrap().count(), 0);
    }

    /// The partial files in `dir`, by name.
    fn partial_files(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".part"))
            .collect();
        names.sort();
        names
    }

    #[test]
    fn a_file_written_whole_replaces_the_last_and_leaves_nothing_beside_it() {
        let dir = dir();
        let path = dir.0.join("kept.json");
        write_whole(&path, b"first", Mode::Default).unwrap();
        write_whole(&path, b"second", Mode::Default).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"second");
        assert_eq!(std::fs::read_dir(&dir.0).unwrap().count(), 1);
    }

    #[test]
    fn two_writers_at_once_leave_one_whole_file_never_a_mix() {
        let dir = dir();
        let path = dir.0.join("kept");
        // Each writer's bytes all alike and of a length of their own: a mix
        // of two would show.
        let contents: Vec<Vec<u8>> = (1..=8u8)
            .map(|n| vec![n; 16 * 1024 * usize::from(n)])
            .collect();
        write_whole(&path, &contents[0], Mode::Default).unwrap();
        let writing = std::sync::atomic::AtomicUsize::new(contents.len());
        let mut mixed = 0;
        std::thread::scope(|scope| {
            for content in &contents {
                let (path, writing) = (&path, &writing);
                scope.spawn(move || {
                    for _ in 0..10 {
                        write_whole(path, content, Mode::Default).unwrap();
                    }
                    writing.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                });
            }
            while writing.load(std::sync::atomic::Ordering::Relaxed) > 0 {
                if !contents.contains(&std::fs::read(&path).unwrap()) {
                    mixed += 1;
                }
            }
        });
        assert_eq!(mixed, 0, "reads that found no writer's bytes whole");
        assert!(contents.contains(&std::fs::read(&path).unwrap()));
        assert_eq!(partial_files(&dir.0), Vec::<String>::new());
    }

    #[test]
    fn a_writer_stopped_before_the_rename_leaves_the_old_file_whole() {
        let dir = dir();
        let path = dir.0.join("kept");
        write_whole(&path, b"old", Mode::Default).unwrap();
        // As a writer killed once its partial file is on disk, before the
        // rename: unwound from there, so that nothing after runs. Unwind
        // safe: after it, the test reads the disk only, no state in memory.
        let stop: &dyn Fn() = &|| std::panic::resume_unwind(Box::new("stopped"));
        let stopped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            write_whole_with(&path, b"new", Mode::Default, Some(stop))
        }));
        assert!(stopped.is_err(), "the writer went on to the rename");
        assert_eq!(std::fs::read(&path).unwrap(), b"old");
        // Its partial file stays, and the next writer never writes into it.
        let left = partial_files(&dir.0);
        assert_eq!(left.len(), 1, "{left:?}");
        assert_eq!(std::fs::read(dir.0.join(&left[0])).unwrap(), b"new");
        write_whole(&path, b"newer", Mode::Default).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"newer");
        assert_eq!(partial_files(&dir.0), left);
        assert_eq!(std::fs::read(dir.0.join(&left[0])).unwrap(), b"new");
    }

    #[cfg(unix)]
    #[test]
    fn a_private_file_is_the_users_alone_whatever_was_left_beside_it() {
        use std::os::unix::fs::PermissionsExt;
        let dir = dir();
        let path = dir.0.join("tokens.json");
        let mode = |path: &Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
        // The old file and partial files under the names writers once used,
        // all readable by others.
        let left = ["tokens.part", "tokens.json.part"].map(|name| dir.0.join(name));
        for file in left.iter().chain([&path]) {
            std::fs::write(file, "left").unwrap();
            std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o644)).unwrap();
        }
        write_whole(&path, b"secret", Mode::Private).unwrap();
        assert_eq!(mode(&path), 0o600, "{:o}", mode(&path));
        assert_eq!(std::fs::read(&path).unwrap(), b"secret");
        for file in &left {
            assert_eq!(std::fs::read(file).unwrap(), b"left", "{}", file.display());
        }
    }

    #[test]
    fn a_write_that_fails_leaves_the_path_as_it_was_and_nothing_beside_it() {
        let dir = dir();
        // A folder where the file goes: the rename fails.
        let path = dir.0.join("kept");
        std::fs::create_dir(&path).unwrap();
        assert!(write_whole(&path, b"new", Mode::Default).is_err());
        assert!(path.is_dir());
        assert_eq!(std::fs::read_dir(&dir.0).unwrap().count(), 1);
        let no_file = write_whole(&dir.0.join(".."), b"new", Mode::Default).unwrap_err();
        assert_eq!(no_file.kind(), io::ErrorKind::InvalidInput);
    }

    /// The names in `dir`, sorted.
    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn the_sweep_removes_every_kind_of_partial_file_left() {
        let dir = dir();
        // Stopped before its rename, as a crash leaves it.
        let stop: &dyn Fn() = &|| std::panic::resume_unwind(Box::new("stopped"));
        let path = dir.0.join("tokens.json");
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            write_whole_with(&path, b"secret", Mode::Private, Some(stop))
        }));
        assert_eq!(partial_files(&dir.0).len(), 1);
        // The names older builds wrote aside under, beside each file kept.
        let older = [
            "settings.part",
            "connect.json.part",
            "media-user-token.part",
            "0123456789abcdef.part",
            "p.1a2b-00ff00ff00ff00ff.part",
            "playlist-p.AbC.json.part.3",
            "index.part.4242.0",
        ];
        for name in older {
            std::fs::write(dir.0.join(name), "left").unwrap();
        }
        assert_eq!(sweep_partials(&dir.0), older.len() + 1);
        assert_eq!(names(&dir.0), Vec::<String>::new());
    }

    #[test]
    fn the_sweep_leaves_every_other_file_and_the_folders_below() {
        let dir = dir();
        let kept = [
            "settings.toml",
            "connect.json",
            "media-user-token",
            "a.partial",
            "notes.part.txt",
            "index.part.",
            ".part",
        ];
        for name in kept {
            std::fs::write(dir.0.join(name), "kept").unwrap();
        }
        // A folder, and a partial file inside it: not below `dir`.
        let below = dir.0.join("browser-profile.part");
        std::fs::create_dir(&below).unwrap();
        std::fs::write(below.join("settings.part"), "kept").unwrap();
        assert_eq!(sweep_partials(&dir.0), 0);
        let mut left: Vec<&str> = kept.into_iter().chain(["browser-profile.part"]).collect();
        left.sort_unstable();
        assert_eq!(names(&dir.0), left);
        assert_eq!(names(&below), ["settings.part"]);
    }

    #[cfg(unix)]
    #[test]
    fn the_sweep_leaves_a_link_named_as_a_partial_file_and_what_it_leads_to() {
        let dir = dir();
        let target = dir.0.join("settings.toml");
        std::fs::write(&target, "kept").unwrap();
        let link = dir.0.join("settings.part");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert_eq!(sweep_partials(&dir.0), 0, "a link is no partial file");
        assert!(link.symlink_metadata().is_ok());
        assert_eq!(std::fs::read(&target).unwrap(), b"kept");
    }

    #[test]
    fn the_sweep_of_a_folder_missing_or_of_a_file_removes_nothing() {
        let dir = dir();
        assert_eq!(sweep_partials(&dir.0.join("missing")), 0);
        let file = dir.0.join("settings.part");
        std::fs::write(&file, "kept").unwrap();
        assert_eq!(sweep_partials(&file), 0);
        assert!(file.exists());
    }
}
