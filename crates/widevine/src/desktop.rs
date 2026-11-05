//! The public handles on the desktop: the one CDM instance of the process, its sessions, and
//! shutting it down (§2, §5, §8).

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::library::{self, Library};
use crate::runtime::{Deadlines, Runtime, SessionKey};
use crate::{Error, Protected};

/// The process's one loader.
static LOADER: Loader = Loader::new();

/// A handle to the process's one CDM instance, loaded and initialised.
///
/// Copying it copies the handle, not the instance. Every call goes to the thread named `cdm`,
/// which owns the instance; callers on any thread wait for its answer.
#[derive(Debug, Clone, Copy)]
pub struct Cdm {
    _private: (),
}

impl Cdm {
    /// Loads the CDM library at `path` and initialises the process's instance of it.
    ///
    /// The library is opened with `RTLD_NOW | RTLD_LOCAL` and never unloaded. Loading again
    /// returns the same instance. After a failed initialisation, the next call creates a new
    /// instance from the library already open. Blocks up to ten seconds while the CDM
    /// initialises.
    ///
    /// # Errors
    ///
    /// [`Error::LibraryNotOpened`], [`Error::NotACdm`] naming the missing function,
    /// [`Error::InterfaceNotSupported`], [`Error::InitialisationFailed`],
    /// [`Error::InitialisationTimedOut`], [`Error::ThreadNotStarted`],
    /// [`Error::AnotherCdmLoaded`] when another path is loaded already, and
    /// [`Error::ShutDown`] after [`shutdown`].
    pub fn load(path: &Path) -> Result<Cdm, Error> {
        LOADER.load(
            path,
            |path| library::open(path, library::cpp_calls()),
            Deadlines::SPECIFIED,
        )?;
        Ok(Cdm { _private: () })
    }

    /// Opens a temporary session with CENC init data (a Widevine `pssh` box, such as
    /// [`pssh_for_key`](crate::pssh_for_key)'s): the session and its licence request, the
    /// challenge to send to the service's licence server.
    ///
    /// Blocks up to ten seconds while the CDM makes its request. Dropping the session closes
    /// it.
    ///
    /// # Errors
    ///
    /// [`Error::InitDataRefused`] with the CDM's message, [`Error::NotProvisioned`] when the CDM
    /// has no device certificate (nothing is sent anywhere), [`Error::NoLicenceRequest`] when
    /// the deadline passes, [`Error::TooLarge`], [`Error::ShutDown`].
    pub fn open_session(&self, init_data: &[u8]) -> Result<(Session, Vec<u8>), Error> {
        let runtime = LOADER.runtime().ok_or(Error::NotInitialised)?;
        let (key, challenge) = runtime.open_session(init_data)?;
        Ok((Session { runtime, key }, challenge))
    }
}

/// A temporary session of the CDM: it takes one licence, then decrypts the samples of the keys
/// the licence gives.
///
/// It can be used from several threads at once. Dropping it closes it in the background (the
/// keys go with it); how the close went is logged.
pub struct Session {
    runtime: Runtime,
    key: SessionKey,
}

impl Session {
    /// Gives the licence server's answer to the session, and returns once `key_id` is usable.
    ///
    /// Blocks up to ten seconds while the CDM takes the licence. The content keys stay inside
    /// the CDM.
    ///
    /// # Errors
    ///
    /// [`Error::LicenceRefused`] with the CDM's message and system code,
    /// [`Error::KeyNotUsable`] with the key's status, [`Error::LicenceNotTaken`] when the
    /// deadline passes, [`Error::NotProvisioned`], [`Error::SessionClosed`],
    /// [`Error::TooLarge`], [`Error::ShutDown`].
    pub fn give_licence(&self, licence: &[u8], key_id: &[u8; 16]) -> Result<(), Error> {
        self.runtime.give_licence(self.key, licence, key_id)
    }

    /// Decrypts one sample in place: on success `data` holds the cleartext, of the same length.
    ///
    /// Without subsample runs the whole sample is protected; with them, they must add up to
    /// the sample's length. An empty sample has nothing to decrypt and succeeds at once. The
    /// call waits for the CDM, which paces its decrypts: most take tens of microseconds, a few
    /// up to about 200 ms. On error `data` is unchanged.
    ///
    /// # Errors
    ///
    /// [`Error::SubsamplesMismatch`] and [`Error::TooLarge`] before reaching the CDM;
    /// [`Error::NoKey`], [`Error::DecryptionFailed`] and the other statuses of the CDM;
    /// [`Error::KeyExpired`] when the key expired since its licence;
    /// [`Error::WrongLength`]; [`Error::SessionClosed`]; [`Error::ShutDown`].
    pub fn decrypt(&self, data: &mut [u8], sample: &Protected<'_>) -> Result<(), Error> {
        self.runtime.decrypt(self.key, data, sample)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.runtime.close(self.key);
    }
}

impl fmt::Debug for Session {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Session")
            .field("key", &self.key)
            .finish()
    }
}

/// Shuts the CDM down before the process exits (§8).
///
/// Requests waiting, and every request sent afterwards, get [`Error::ShutDown`]. The CDM thread
/// finishes the call under way, closes the sessions still open, destroys the instance, ends the
/// library's module and ends. Returns when the thread has ended, or after five seconds. The
/// library is never unloaded. Calling it again, or without a CDM loaded, does no harm.
pub fn shutdown() {
    LOADER.shutdown();
}

/// Loads the process's one instance: serialises loading, keeps the library's path and the
/// runtime, and refuses everything after shutdown.
pub(crate) struct Loader {
    /// Held for the whole of a load, so that loads happen one at a time.
    loading: Mutex<Loaded>,
    /// Held briefly: shutting down never waits for a load.
    current: Mutex<Current>,
}

#[derive(Default)]
struct Loaded {
    /// The library's path, once opened.
    path: Option<PathBuf>,
    /// Whether its instance is initialised.
    ready: bool,
}

#[derive(Default)]
struct Current {
    runtime: Option<Runtime>,
    shut_down: bool,
}

impl Loader {
    pub(crate) const fn new() -> Loader {
        Loader {
            loading: Mutex::new(Loaded {
                path: None,
                ready: false,
            }),
            current: Mutex::new(Current {
                runtime: None,
                shut_down: false,
            }),
        }
    }

    /// Loads the library at `path` with `open` the first time, and initialises its instance
    /// unless it is initialised already.
    pub(crate) fn load(
        &self,
        path: &Path,
        open: impl FnOnce(&Path) -> Result<Library, Error>,
        deadlines: Deadlines,
    ) -> Result<Runtime, Error> {
        let mut loaded = self.loading.lock().unwrap_or_else(PoisonError::into_inner);
        if self.lock_current().shut_down {
            return Err(Error::ShutDown);
        }
        let wanted = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        match &loaded.path {
            Some(open_path) if *open_path != wanted => {
                return Err(Error::AnotherCdmLoaded(open_path.clone()));
            }
            Some(_) => {}
            None => {
                let library = open(path)?;
                let mut current = self.lock_current();
                if current.shut_down {
                    return Err(Error::ShutDown);
                }
                current.runtime = Some(Runtime::start(library, deadlines)?);
                loaded.path = Some(wanted);
            }
        }
        let runtime = self
            .lock_current()
            .runtime
            .clone()
            .ok_or(Error::HostStopped)?;
        if !loaded.ready {
            runtime.initialise()?;
            loaded.ready = true;
        }
        Ok(runtime)
    }

    /// The runtime, once a library is loaded.
    pub(crate) fn runtime(&self) -> Option<Runtime> {
        self.lock_current().runtime.clone()
    }

    pub(crate) fn shutdown(&self) {
        let runtime = {
            let mut current = self.lock_current();
            current.shut_down = true;
            current.runtime.clone()
        };
        if let Some(runtime) = runtime
            && !runtime.shutdown()
        {
            crate::log::line(format_args!(
                "shutting down: the CDM thread did not end in time"
            ));
        }
    }

    fn lock_current(&self) -> MutexGuard<'_, Current> {
        self.current.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
