// Belongs in: a new file crates/desktop/src/backend/tests.rs, declared at the end of
// crates/desktop/src/backend.rs with:
//
//     #[cfg(test)]
//     mod tests;
//
// It would be the first test of `backend.rs`. Not built here (desktop needs every crate of the
// workspace); written against the code at 46e7a5a. Passes there by reading
// (`backend.rs:621-648`).

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use super::write_private;

/// A secret (Apple's user token, SoundCloud's tokens and cookie, Navidrome's sign-in, the
/// settings) is written readable by the user alone, in a folder only the user opens, and
/// replaced whole: AGENTS.md's "Tokens in user-only files (`desktop::backend::write_private`)",
/// which no test checked.
///
/// Kills `backend.rs:634:5 replace write_private -> std::io::Result<()> with Ok(())` and
/// `backend.rs:623:5 replace private_dir -> std::io::Result<()> with Ok(())`.
#[test]
fn a_secret_is_written_for_the_user_alone_and_whole() {
    let dir = std::env::temp_dir().join(format!("desktop-private-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let folder = dir.join("apple-music");
    let path = folder.join("media-user-token");
    let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;

    write_private(&path, b"first").unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"first");
    assert_eq!(mode(&path), 0o600);
    assert_eq!(mode(&folder), 0o700);

    // Again: replaced whole, nothing left beside it.
    write_private(&path, b"second, and longer").unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"second, and longer");
    assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 1);

    // A folder opened to others meanwhile is the user's alone again.
    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o755)).unwrap();
    write_private(&path, b"third").unwrap();
    assert_eq!(mode(&folder), 0o700);

    std::fs::remove_dir_all(&dir).unwrap();
}
