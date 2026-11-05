//! Seeded random sequences: requests from several threads, a fake CDM answering at random (late,
//! twice, never, wrongly, for sessions unknown or closed), and shutdown at a random point.
//!
//! Each sequence is checked for no panic, no hang, every request answered exactly once, and
//! shutdown ending every waiter. A hang shows as the CDM thread's turns growing beyond what the
//! work explains: steps are counted, never a clock.

use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

use super::fake::{Call, Fake, Step, unmask};
use super::{Rng, key, start};
use crate::boundary::{MESSAGE_INDIVIDUALIZATION_REQUEST, MESSAGE_LICENCE_REQUEST};
use crate::runtime::{Deadlines, Runtime};
use crate::{Error, Protected, Scheme, pssh_for_key};

const SEQUENCES: u64 = 2_400;
const DRIVERS: u64 = 4;
const WORKERS: u64 = 3;

/// Short enough that thousands of sequences, many of them waiting for a CDM that never
/// answers, take seconds.
const DEADLINES: Deadlines = Deadlines {
    initialise: Duration::from_millis(3),
    session_request: Duration::from_millis(2),
    licence: Duration::from_millis(2),
    close: Duration::from_millis(1),
    shutdown: Duration::from_secs(10),
};

#[test]
fn thousands_of_random_sequences_answer_every_request_once_and_end() {
    thread::scope(|scope| {
        for driver in 0..DRIVERS {
            scope.spawn(move || {
                for index in (driver..SEQUENCES).step_by(DRIVERS as usize) {
                    run_sequence(0x5e9_0000 + index);
                }
            });
        }
    });
}

fn run_sequence(seed: u64) {
    let fake = Fake::new(chaotic_cdm(seed));
    let runtime = start(&fake, DEADLINES);
    let mut rng = Rng::new(seed ^ 0xd1ce);
    let mut requests = 0;
    for _ in 0..3 {
        requests += 1;
        if runtime.initialise().is_ok() {
            break;
        }
    }
    let shutdown_early = rng.chance(50);
    let yields = rng.below(300);
    let seeds: Vec<u64> = (0..WORKERS).map(|_| rng.next()).collect();
    let mut outcomes = thread::scope(|scope| {
        let runtime = &runtime;
        let workers: Vec<_> = seeds
            .iter()
            .map(|&seed| scope.spawn(move || worker(runtime, seed)))
            .collect();
        if shutdown_early {
            for _ in 0..yields {
                thread::yield_now();
            }
            assert!(
                runtime.shutdown(),
                "seed {seed:#x}: shutdown ended the thread"
            );
        }
        workers
            .into_iter()
            .map(|worker| worker.join().expect("no worker panics"))
            .collect::<Vec<_>>()
    });
    if !shutdown_early {
        assert!(
            runtime.shutdown(),
            "seed {seed:#x}: shutdown ended the thread"
        );
    }
    assert_eq!(
        runtime.open_session(b"after").map(|_| ()),
        Err(Error::ShutDown),
        "seed {seed:#x}"
    );

    let counters = &runtime.shared().counters;
    assert!(
        !counters.panicked.load(Ordering::Relaxed),
        "seed {seed:#x}: the CDM thread panicked"
    );
    for outcome in &outcomes {
        assert!(
            outcome.lost.is_empty(),
            "seed {seed:#x}: {:?}",
            outcome.lost
        );
    }
    let accepted = counters.accepted.load(Ordering::Relaxed);
    let answered = counters.answered.load(Ordering::Relaxed);
    assert_eq!(
        accepted, answered,
        "seed {seed:#x}: every request answered exactly once"
    );
    requests += outcomes
        .iter_mut()
        .map(|outcome| outcome.requests)
        .sum::<u64>();
    let timers = counters.timers_set.load(Ordering::Relaxed);
    let turns = counters.turns.load(Ordering::Relaxed);
    let bound = 10 * (requests + timers + 10);
    assert!(
        turns <= bound,
        "seed {seed:#x}: {turns} turns for {requests} requests and {timers} timers"
    );
    assert_eq!(
        fake.buffers_held.load(Ordering::Relaxed),
        0,
        "seed {seed:#x}: a CDM buffer leaked"
    );
    assert_eq!(
        fake.calls().last(),
        Some(&Call::ModuleDeinitialised),
        "seed {seed:#x}"
    );
}

/// What one worker saw.
struct Outcome {
    requests: u64,
    /// Requests whose answer was lost: dropped unanswered.
    lost: Vec<Error>,
}

/// Opens sessions, gives licences, decrypts and closes at random.
fn worker(runtime: &Runtime, seed: u64) -> Outcome {
    let mut rng = Rng::new(seed);
    let mut sessions: Vec<u64> = Vec::new();
    let mut outcome = Outcome {
        requests: 0,
        lost: Vec::new(),
    };
    let mut note = |result: Result<(), Error>| {
        if let Err(error @ Error::HostStopped) = result {
            outcome.lost.push(error);
        }
    };
    for _ in 0..4 + rng.below(8) {
        let session = if sessions.is_empty() || rng.chance(10) {
            rng.next()
        } else {
            sessions[rng.below(sessions.len() as u64) as usize]
        };
        match rng.below(10) {
            0..=2 => match runtime.open_session(&pssh_for_key(&key(rng.byte()))) {
                Ok((opened, challenge)) => {
                    assert!(!challenge.is_empty());
                    sessions.push(opened);
                }
                Err(error) => note(Err(error)),
            },
            3 | 4 => {
                let key_id = key(rng.below(3) as u8);
                note(runtime.give_licence(session, &key_id, &key_id));
            }
            5..=8 => note(decrypt(runtime, session, &mut rng)),
            _ => {
                sessions.retain(|&open| open != session);
                runtime.close(session);
            }
        }
        outcome.requests += 1;
    }
    for session in sessions {
        runtime.close(session);
        outcome.requests += 1;
    }
    outcome
}

fn decrypt(runtime: &Runtime, session: u64, rng: &mut Rng) -> Result<(), Error> {
    let size = rng.below(48) as u32;
    let runs: Vec<(u32, u32)> = match rng.below(10) {
        0..=5 => Vec::new(),
        6..=8 => {
            let clear = rng.below(u64::from(size) + 1) as u32;
            vec![(clear, size - clear)]
        }
        _ => vec![(size, 1)],
    };
    let clear: Vec<u8> = (0..size).map(|_| rng.byte()).collect();
    let mut data = unmask(&clear, &runs);
    let scheme = if rng.chance(50) {
        Scheme::Cenc
    } else {
        Scheme::Cbcs { crypt: 1, skip: 9 }
    };
    let key_id = key(rng.below(3) as u8);
    let sample = Protected {
        key_id: &key_id,
        iv: &[3; 16],
        subsamples: &runs,
        scheme,
    };
    let result = runtime.decrypt(session, &mut data, &sample);
    match &result {
        Ok(()) => assert_eq!(data, clear, "a decrypt's cleartext lands in place"),
        Err(_) => assert_eq!(
            data,
            unmask(&clear, &runs),
            "a failed decrypt leaves the sample unchanged"
        ),
    }
    result
}

/// A CDM that answers each call at random: at once or later, in either order, twice, never,
/// wrongly, and that calls back now and then about promises and sessions unknown or closed.
fn chaotic_cdm(seed: u64) -> impl FnMut(&Call) -> Vec<Step> + Send + 'static {
    let mut rng = Rng::new(seed);
    let mut names: Vec<Vec<u8>> = Vec::new();
    let mut last_promise = 0;
    move |call| {
        let mut steps = match call {
            Call::Initialize { .. } => match rng.below(20) {
                0 => vec![Step::Initialized(false)],
                1 => Vec::new(),
                2 => vec![Step::Later {
                    delay_ms: 1,
                    steps: vec![Step::Initialized(true)],
                }],
                _ => vec![Step::Initialized(true)],
            },
            Call::CreateSession { promise, .. } => {
                last_promise = *promise;
                let name = format!("s{}", names.len()).into_bytes();
                names.push(name.clone());
                let named = Step::NewSession {
                    promise: *promise,
                    session: name.clone(),
                };
                let request = Step::Message {
                    session: name.clone(),
                    kind: MESSAGE_LICENCE_REQUEST,
                    message: b"challenge".to_vec(),
                };
                let answer = match rng.below(10) {
                    0 => vec![reject(*promise)],
                    1 => vec![
                        named,
                        Step::Message {
                            session: name,
                            kind: MESSAGE_INDIVIDUALIZATION_REQUEST,
                            message: b"cert".to_vec(),
                        },
                    ],
                    2 => Vec::new(),
                    3 => vec![named],
                    4 => vec![request, named],
                    _ => vec![named, request],
                };
                maybe_later(&mut rng, answer)
            }
            Call::UpdateSession {
                promise,
                session,
                response,
            } => {
                last_promise = *promise;
                let statuses = |status| Step::Keys {
                    session: session.clone(),
                    keys: vec![(response.clone(), status)],
                };
                let answer = match rng.below(10) {
                    0 => vec![reject(*promise)],
                    1 => Vec::new(),
                    2 => vec![Step::Resolve(*promise)],
                    3 => vec![statuses(1 + rng.below(7) as u32), Step::Resolve(*promise)],
                    4 => vec![Step::Closed {
                        session: session.clone(),
                    }],
                    5 => vec![
                        statuses(5),
                        Step::Resolve(*promise),
                        Step::Later {
                            delay_ms: rng.below(3) as i64,
                            steps: vec![statuses(0)],
                        },
                    ],
                    6 => vec![
                        Step::Resolve(*promise),
                        statuses(0),
                        Step::Resolve(*promise),
                    ],
                    _ => vec![statuses(0), Step::Resolve(*promise)],
                };
                maybe_later(&mut rng, answer)
            }
            Call::CloseSession { promise, session } => {
                let closed = Step::Closed {
                    session: session.clone(),
                };
                let answer = match rng.below(6) {
                    0 => Vec::new(),
                    1 => vec![reject(*promise)],
                    2 => vec![closed, Step::Resolve(*promise)],
                    _ => vec![Step::Resolve(*promise), closed],
                };
                maybe_later(&mut rng, answer)
            }
            Call::Decrypt(decrypt) => match rng.below(20) {
                0 => vec![Step::Status(1 + rng.below(7) as u32)],
                1 => vec![Step::OutputLength(decrypt.size + 1)],
                _ => Vec::new(),
            },
            _ => Vec::new(),
        };
        if rng.chance(20) {
            steps.push(noise(&mut rng, &names, last_promise));
        }
        steps
    }
}

fn reject(promise: u32) -> Step {
    Step::Reject {
        promise,
        exception: 3,
        system_code: 1,
        message: "no".to_owned(),
    }
}

fn maybe_later(rng: &mut Rng, steps: Vec<Step>) -> Vec<Step> {
    if rng.chance(30) {
        vec![Step::Later {
            delay_ms: rng.below(4) as i64,
            steps,
        }]
    } else {
        steps
    }
}

/// A callback nobody asked for.
fn noise(rng: &mut Rng, names: &[Vec<u8>], last_promise: u32) -> Step {
    let session = if names.is_empty() || rng.chance(20) {
        b"never named".to_vec()
    } else {
        names[rng.below(names.len() as u64) as usize].clone()
    };
    let promise = last_promise
        .wrapping_add(rng.below(5) as u32)
        .wrapping_sub(2);
    match rng.below(11) {
        0 => Step::Resolve(promise),
        1 => reject(promise),
        2 => Step::NewSession { promise, session },
        3 => Step::Keys {
            session,
            keys: vec![(key(rng.below(3) as u8).to_vec(), rng.below(9) as u32)],
        },
        4 => Step::Message {
            session,
            kind: rng.below(6) as u32,
            message: b"noise".to_vec(),
        },
        5 => Step::Closed { session },
        6 => Step::Expiration { session, time: 0.0 },
        7 => Step::RequestStorageId(rng.below(3) as u32),
        8 => Step::Unanswered(rng.below(8) as u32),
        9 => Step::KeyStatusResolved(promise),
        _ => Step::Later {
            delay_ms: 0,
            steps: vec![Step::Later {
                delay_ms: rng.below(2) as i64,
                steps: vec![Step::Mark("set by a timer")],
            }],
        },
    }
}
