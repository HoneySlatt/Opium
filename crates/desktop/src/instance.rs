//! One Opium per profile
//! ([ADR-0043](../../../docs/adr/0043-one-instance-per-profile-held-by-a-lock.md)): an
//! exclusive `flock` on [`LOCK_FILE`] in the config folder, taken before
//! anything else and held until the process ends. Two instances on one
//! profile would each rewrite its files whole from their own copies, and one
//! would sweep the other's downloads.
//!
//! The config folder is the profile: an instance started on another one, an
//! agent's throwaway profile say, runs beside, touching none of its files.
//! The kernel lets the lock go when the process ends in any way, a crash or
//! a kill included, so a stale lock never keeps Opium from starting. A
//! config folder on a network file system that ignores `flock` is not
//! guarded.

use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

/// The lock's file, in the config folder. Empty: only its lock matters.
pub const LOCK_FILE: &str = "instance.lock";

/// The lock held: no other Opium runs on this profile while it lives.
/// Dropping it lets another start; keep it for the process's life.
#[derive(Debug)]
pub struct Instance {
    // Open for as long as the lock is held: closing it releases the lock.
    _file: File,
}

/// Why this process may not run as its profile's Opium.
#[derive(Debug)]
pub enum Refused {
    /// Another process holds the lock: Opium already runs on this profile.
    Running,
    /// Neither `XDG_CONFIG_HOME` nor `HOME` names an absolute folder
    /// ([`crate::folders::config`]).
    NoConfigFolder,
    /// The lock couldn't be made or taken: the folder can't be written, say.
    Failed {
        /// The lock's file.
        path: PathBuf,
        /// What failed: making the folder, opening the file, or `flock`.
        error: io::Error,
    },
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Running => write!(f, "Opium already runs on this profile"),
            Self::NoConfigFolder => write!(
                f,
                "there is no folder to keep the settings in: neither XDG_CONFIG_HOME nor HOME \
                 names an absolute folder"
            ),
            Self::Failed { path, error } => {
                write!(f, "couldn't lock {}: {error}", path.display())
            }
        }
    }
}

impl std::error::Error for Refused {}

/// Take the lock of the profile the environment names: [`LOCK_FILE`] in
/// [`crate::folders::config`].
///
/// # Errors
///
/// [`Refused::Running`] while another process holds it,
/// [`Refused::NoConfigFolder`] without a config folder, [`Refused::Failed`]
/// when the folder or the file can't be made, or `flock` fails otherwise.
pub fn take() -> Result<Instance, Refused> {
    take_in(&crate::folders::config().ok_or(Refused::NoConfigFolder)?)
}

/// Take the lock in `dir`, as [`take`] does in the config folder; `dir` is
/// made if missing, for the user only (`0700`).
///
/// A second take while the first [`Instance`] lives fails, in this process
/// as in any other: `flock` locks belong to each opening of the file, not to
/// the process.
///
/// # Errors
///
/// As [`take`], but for [`Refused::NoConfigFolder`].
pub fn take_in(dir: &Path) -> Result<Instance, Refused> {
    let path = dir.join(LOCK_FILE);
    let failed = |error| Refused::Failed {
        path: path.clone(),
        error,
    };
    // Made if missing; an existing folder's permissions are left to the
    // backend, which lets a failure pass: a file system that keeps none
    // must not stop Opium starting.
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
        .map_err(failed)?;
    // The standard library opens with `O_CLOEXEC`: a program Opium starts,
    // the sign-in browser say, doesn't inherit the file, which would hold
    // the lock on after Opium ended.
    let file = match OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(&path)
    {
        // A file left read-only (a profile restored from a backup, say):
        // `flock` needs no write access, so it is locked as it is.
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
            File::open(&path).map_err(|_| failed(e))?
        }
        opened => opened.map_err(failed)?,
    };
    // SAFETY: `flock` takes a descriptor and flags and touches no memory of
    // ours; the descriptor is `file`'s own, open for the whole call.
    let locked = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if locked == 0 {
        return Ok(Instance { _file: file });
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::EWOULDBLOCK) {
        Err(Refused::Running)
    } else {
        Err(failed(error))
    }
}
