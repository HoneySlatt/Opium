//! Songs downloaded to play offline, kept sealed so that only Opium plays
//! them.

#[cfg(feature = "keyring")]
pub mod keyring;
pub mod queue;
pub mod sealed;
pub mod store;

/// `mutex` locked; a thread that panicked holding it left what it guards
/// whole, so its poison is ignored.
fn lock<T>(mutex: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
