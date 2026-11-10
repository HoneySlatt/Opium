//! Files put aside rather than lost: a file that doesn't read is moved out
//! of the way, under a name no other file has, for the user to recover.

use std::fs::OpenOptions;
use std::io;
use std::path::{Path, PathBuf};

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
}
