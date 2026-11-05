//! What the host logs: one line on standard error per event worth knowing, naming its step.
//! Never a challenge's, a licence's or a sample's bytes.

use std::fmt;

/// Writes `widevine: <what>` on standard error. A failed write is ignored: logging never
/// fails or panics the host.
#[cfg(not(test))]
pub(crate) fn line(what: fmt::Arguments<'_>) {
    use std::io::Write;
    let _ = writeln!(std::io::stderr(), "widevine: {what}");
}

/// In tests, through the macro the test harness captures, so that each test's lines stay with
/// it; tests can afford its panic on a failed write.
#[cfg(test)]
pub(crate) fn line(what: fmt::Arguments<'_>) {
    eprintln!("widevine: {what}");
}
