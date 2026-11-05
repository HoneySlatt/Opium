//! Every way the host can fail (§9 of the specification), each naming its step.

use std::path::PathBuf;

/// Why a step of loading the CDM, opening a session, taking a licence or decrypting failed.
///
/// Each case names its step and keeps what the CDM said. Messages never hold a challenge's, a
/// licence's or a sample's bytes.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The library could not be opened; the loader's message.
    #[error("loading the CDM: the library could not be opened: {0}")]
    LibraryNotOpened(String),
    /// The library does not export a function every CDM exports; its name.
    #[error("loading the CDM: not a CDM, the library does not export {0}")]
    NotACdm(&'static str),
    /// The CDM gave no instance of the interface version the host speaks; that version.
    #[error("loading the CDM: interface {0} not supported")]
    InterfaceNotSupported(u32),
    /// The CDM answered its initialisation with a failure.
    #[error("initialising the CDM: initialisation failed")]
    InitialisationFailed,
    /// The CDM did not answer its initialisation within its deadline.
    #[error("initialising the CDM: the CDM did not initialise")]
    InitialisationTimedOut,
    /// A request reached a host whose CDM is not initialised.
    #[error("the CDM is not initialised")]
    NotInitialised,
    /// Another library is loaded already; its path.
    #[error("loading the CDM: another CDM is loaded already, {}", .0.display())]
    AnotherCdmLoaded(PathBuf),
    /// The thread that owns the CDM could not be started; the system's message.
    #[error("starting the CDM thread: {0}")]
    ThreadNotStarted(String),
    /// The thread that owns the CDM ended unexpectedly (a panic), before or while answering:
    /// no request is taken any more.
    #[error("the CDM thread ended")]
    HostStopped,
    /// This platform has no CDM to load (Android, where MediaDrm decrypts).
    #[error("no CDM on this platform")]
    NoCdmOnThisPlatform,
    /// The CDM has no device certificate and asked for one; nothing was sent anywhere.
    #[error("opening a session: not provisioned, the CDM has no device certificate")]
    NotProvisioned,
    /// The CDM refused a session's init data.
    #[error(
        "opening a session: the CDM refused the init data: {message} ({exception}, system code {system_code})"
    )]
    InitDataRefused {
        /// The kind of rejection, as EME names it (`TypeError`, `NotSupportedError`, …).
        exception: &'static str,
        /// The CDM's own error code.
        system_code: u32,
        /// The CDM's message.
        message: String,
    },
    /// The CDM made no licence request within its deadline.
    #[error("opening a session: the CDM made no licence request")]
    NoLicenceRequest,
    /// The CDM refused the licence.
    #[error(
        "taking the licence: the CDM refused it: {message} ({exception}, system code {system_code})"
    )]
    LicenceRefused {
        /// The kind of rejection, as EME names it (`TypeError`, `NotSupportedError`, …).
        exception: &'static str,
        /// The CDM's own error code.
        system_code: u32,
        /// The CDM's message.
        message: String,
    },
    /// The CDM did not take the licence, or did not make the needed key usable, in time.
    #[error("taking the licence: the CDM did not take the licence")]
    LicenceNotTaken,
    /// The licence gave the needed key a final status other than usable; that status.
    #[error("taking the licence: the needed key is not usable, its status is {status}")]
    KeyNotUsable {
        /// The key's status: `internal error`, `expired`, `output restricted`,
        /// `output downscaled` or `released`.
        status: &'static str,
    },
    /// The sample's key has expired since its licence was taken: a new session needs a new
    /// licence.
    #[error("decrypting: key expired")]
    KeyExpired,
    /// The CDM closed the session.
    #[error("the CDM closed the session")]
    SessionClosed,
    /// The subsample runs do not add up to the sample's size; the CDM was not called.
    #[error(
        "decrypting: the subsample runs add up to {runs} bytes, not the sample's {sample} bytes"
    )]
    SubsamplesMismatch {
        /// The sample's size, in bytes.
        sample: usize,
        /// What the runs add up to, in bytes.
        runs: u64,
    },
    /// Data too large for the CDM's 32-bit sizes; the CDM was not called.
    #[error("{what} is too large for the CDM: {size}")]
    TooLarge {
        /// What was too large: init data, a licence, a sample or its subsample runs.
        what: &'static str,
        /// Its size, in bytes or in runs.
        size: usize,
    },
    /// The CDM has no key for this content (`kNoKey`).
    #[error("decrypting: no key for this content")]
    NoKey,
    /// The CDM could not decrypt the sample (`kDecryptError`).
    #[error("decrypting: decryption failed")]
    DecryptionFailed,
    /// The CDM answered a decrypt with `kInitializationError`.
    #[error("decrypting: the CDM reported an initialisation error")]
    DecryptorInitialisation,
    /// The CDM answered a decrypt with `kNeedMoreData`.
    #[error("decrypting: the CDM needs more data")]
    NeedMoreData,
    /// The CDM answered a decrypt with `kDecodeError`.
    #[error("decrypting: the CDM reported a decoding error")]
    DecodeError,
    /// The CDM answered a decrypt with `kDeferredInitialization`.
    #[error("decrypting: the CDM deferred its initialisation")]
    DeferredInitialisation,
    /// The CDM answered a decrypt with a status the interface does not define; its value.
    #[error("decrypting: the CDM answered with unknown status {0}")]
    UnknownStatus(u32),
    /// The CDM's cleartext is not the sample's length; the caller's bytes are unchanged.
    #[error("decrypting: the cleartext is {got} bytes, not the sample's {expected}")]
    WrongLength {
        /// The sample's size, in bytes.
        expected: usize,
        /// The cleartext's size, in bytes.
        got: usize,
    },
    /// The host is shut down, or was shutting down when the request waited.
    #[error("shut down")]
    ShutDown,
}

/// The name EME gives a promise's rejection.
#[cfg(not(target_os = "android"))]
pub(crate) fn exception_name(exception: u32) -> &'static str {
    match exception {
        0 => "TypeError",
        1 => "NotSupportedError",
        2 => "InvalidStateError",
        3 => "QuotaExceededError",
        _ => "unknown exception",
    }
}

/// The error for a decrypt's status, or `None` for success (`kSuccess`, 0).
#[cfg(not(target_os = "android"))]
pub(crate) fn decrypt_status_error(status: u32) -> Option<Error> {
    match status {
        0 => None,
        1 => Some(Error::NeedMoreData),
        2 => Some(Error::NoKey),
        3 => Some(Error::DecryptorInitialisation),
        4 => Some(Error::DecryptionFailed),
        5 => Some(Error::DecodeError),
        6 => Some(Error::DeferredInitialisation),
        other => Some(Error::UnknownStatus(other)),
    }
}
