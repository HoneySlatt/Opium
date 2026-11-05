//! Probes the browser's CDM without an account (§11 of the specification): lists the
//! candidates, loads and initialises the CDM, opens a session for a made-up key id to get its
//! licence request, then shuts down. The host's log, on standard error, tells which timers and
//! storage requests the CDM made meanwhile.
//!
//! It loads the real CDM: run it only with the owner's go.
//!
//! ```sh
//! cargo run -p widevine --example probe [path/to/libwidevinecdm.so]
//! ```
//!
//! Without a path, it uses the CDM `widevine::find_cdm` chooses. It prints sizes and times,
//! never the licence request's bytes.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

/// A key id no service uses: the CDM makes a licence request for it all the same.
const MADE_UP_KEY_ID: [u8; 16] = *b"opium-probe-key!";

fn main() -> ExitCode {
    println!("candidates:");
    for candidate in widevine::cdm_candidates() {
        let version = candidate.version.as_deref().unwrap_or("no version");
        match &candidate.set_aside {
            None => println!("  {} ({version})", candidate.path.display()),
            Some(reason) => println!(
                "  {} ({version}): set aside, {reason}",
                candidate.path.display()
            ),
        }
    }
    let Some(path) = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .or_else(widevine::find_cdm)
    else {
        println!("no CDM found");
        return ExitCode::FAILURE;
    };
    println!("loading {}", path.display());

    let started = Instant::now();
    let cdm = match widevine::Cdm::load(&path) {
        Ok(cdm) => cdm,
        Err(error) => {
            println!("{error}");
            widevine::shutdown();
            return ExitCode::FAILURE;
        }
    };
    println!("loaded and initialised in {:.2} ms", milliseconds(started));

    let init_data = widevine::pssh_for_key(&MADE_UP_KEY_ID);
    let started = Instant::now();
    let outcome = match cdm.open_session(&init_data) {
        Ok((session, request)) => {
            println!(
                "a session's licence request: {} bytes, in {:.2} ms",
                request.len(),
                milliseconds(started)
            );
            drop(session);
            ExitCode::SUCCESS
        }
        Err(error) => {
            println!("{error}");
            ExitCode::FAILURE
        }
    };

    let started = Instant::now();
    widevine::shutdown();
    println!("shut down in {:.2} ms", milliseconds(started));
    outcome
}

fn milliseconds(since: Instant) -> f64 {
    since.elapsed().as_secs_f64() * 1e3
}
