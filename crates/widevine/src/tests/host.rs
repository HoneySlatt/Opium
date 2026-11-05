//! The host's state machine, driven by the fake CDM through the C boundary, in the orders of
//! callbacks the specification's §14 lists.

use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use super::fake::{Call, DecryptCall, Fake, Gate, Step, unmask, well_behaved};
use super::{QUICK, key, ready, start};
use crate::boundary::{
    HOST_CALLS, MESSAGE_INDIVIDUALIZATION_REQUEST, MESSAGE_LICENCE_REQUEST, SCHEME_CBCS,
    SCHEME_CENC, UNANSWERED_FILE_IO, UNANSWERED_METRICS, UNANSWERED_QUERY_OUTPUT_PROTECTION,
};
use crate::runtime::{Deadlines, Runtime};
use crate::{Error, Protected, Scheme, pssh_for_key};

/// Waits are bounded so that a broken host fails the test instead of hanging it; no test
/// asserts a time.
const HANG_GUARD: Duration = Duration::from_secs(20);

/// A script that answers as `custom` says, and as a well-behaved CDM otherwise.
fn script(
    mut custom: impl FnMut(&Call) -> Option<Vec<Step>> + Send + 'static,
) -> impl FnMut(&Call) -> Vec<Step> + Send + 'static {
    let mut base = well_behaved();
    move |call| custom(call).unwrap_or_else(|| base(call))
}

fn deadlines(change: impl FnOnce(&mut Deadlines)) -> Deadlines {
    let mut deadlines = QUICK;
    change(&mut deadlines);
    deadlines
}

fn wait_for(signal: &Receiver<()>) {
    signal.recv_timeout(HANG_GUARD).expect("the fake got there");
}

fn ignored(runtime: &Runtime) -> u64 {
    runtime.shared().counters.ignored.load(Ordering::Relaxed)
}

/// A session opened and given the licence for `key(1)`.
fn licensed(runtime: &Runtime) -> u64 {
    let (session, challenge) = runtime.open_session(&pssh_for_key(&key(1))).unwrap();
    assert_eq!(challenge, b"challenge");
    runtime.give_licence(session, &key(1), &key(1)).unwrap();
    session
}

fn cenc(key_id: &[u8; 16]) -> Protected<'_> {
    Protected {
        key_id,
        iv: &[7; 16],
        subsamples: &[],
        scheme: Scheme::Cenc,
    }
}

fn decrypts(fake: &Fake) -> Vec<DecryptCall> {
    fake.calls()
        .into_iter()
        .filter_map(|call| match call {
            Call::Decrypt(decrypt) => Some(decrypt),
            _ => None,
        })
        .collect()
}

fn position(calls: &[Call], wanted: impl Fn(&Call) -> bool) -> usize {
    calls.iter().position(wanted).expect("the call was made")
}

// Loading and initialising.

#[test]
fn the_instance_is_initialised_without_identifier_state_or_secure_codecs() {
    let fake = Fake::new(well_behaved());
    let runtime = ready(&fake);
    assert_eq!(
        runtime.initialise(),
        Ok(()),
        "initialising again is the same instance"
    );
    assert!(runtime.shutdown());
    assert_eq!(
        fake.calls(),
        vec![
            Call::ModuleInitialised,
            Call::Create {
                key_system: b"com.widevine.alpha".to_vec()
            },
            Call::Initialize {
                distinctive_identifier: false,
                persistent_state: false,
                hw_secure_codecs: false
            },
            Call::Destroy,
            Call::ModuleDeinitialised,
        ]
    );
}

#[test]
fn a_failed_initialisation_leaves_no_instance_and_the_next_attempt_makes_one() {
    let mut attempts = 0;
    let fake = Fake::new(script(move |call| match call {
        Call::Initialize { .. } => {
            attempts += 1;
            Some(vec![Step::Initialized(attempts > 1)])
        }
        _ => None,
    }));
    let runtime = start(&fake, QUICK);
    assert_eq!(runtime.initialise(), Err(Error::InitialisationFailed));
    assert_eq!(
        runtime.open_session(b"init").map(|_| ()),
        Err(Error::NotInitialised)
    );
    assert_eq!(runtime.initialise(), Ok(()));
    assert!(runtime.shutdown());
    let calls = fake.calls();
    let creates = calls
        .iter()
        .filter(|call| matches!(call, Call::Create { .. }))
        .count();
    let destroys = calls
        .iter()
        .filter(|call| matches!(call, Call::Destroy))
        .count();
    let modules = calls
        .iter()
        .filter(|call| matches!(call, Call::ModuleInitialised))
        .count();
    assert_eq!((creates, destroys, modules), (2, 2, 1));
    assert!(
        position(&calls, |call| matches!(call, Call::Destroy))
            < calls
                .iter()
                .rposition(|call| matches!(call, Call::Create { .. }))
                .unwrap()
    );
}

#[test]
fn an_initialisation_never_answered_meets_its_deadline() {
    let mut attempts = 0;
    let fake = Fake::new(script(move |call| match call {
        Call::Initialize { .. } => {
            attempts += 1;
            Some(if attempts == 1 {
                Vec::new()
            } else {
                vec![Step::Initialized(true)]
            })
        }
        _ => None,
    }));
    let runtime = start(
        &fake,
        deadlines(|d| d.initialise = Duration::from_millis(30)),
    );
    assert_eq!(runtime.initialise(), Err(Error::InitialisationTimedOut));
    assert_eq!(
        runtime.initialise(),
        Ok(()),
        "a new instance after the one that never answered"
    );
    assert!(runtime.shutdown());
    let calls = fake.calls();
    assert_eq!(
        calls
            .iter()
            .filter(|call| matches!(call, Call::Destroy))
            .count(),
        2
    );
}

fn marks(fake: &Fake) -> Vec<&'static str> {
    fake.calls()
        .into_iter()
        .filter_map(|call| match call {
            Call::Mark(name) => Some(name),
            _ => None,
        })
        .collect()
}

fn storage_answers(fake: &Fake) -> Vec<Call> {
    fake.calls()
        .into_iter()
        .filter(|call| matches!(call, Call::StorageId { .. }))
        .collect()
}

#[test]
fn a_timer_and_a_storage_request_made_inside_the_instances_creation_are_honoured() {
    let fake = Fake::new(script(|call| match call {
        // The instance initialises only once the timer set inside its creation fires.
        Call::Create { .. } => Some(vec![
            Step::Later {
                delay_ms: 1,
                steps: vec![
                    Step::Mark("timer from the creation"),
                    Step::Initialized(true),
                ],
            },
            Step::RequestStorageId(7),
        ]),
        Call::Initialize { .. } => Some(Vec::new()),
        _ => None,
    }));
    let runtime = ready(&fake);
    licensed(&runtime);
    assert!(runtime.shutdown());
    assert_eq!(marks(&fake), ["timer from the creation"]);
    assert_eq!(
        storage_answers(&fake),
        [Call::StorageId {
            version: 7,
            given: false
        }]
    );
    let calls = fake.calls();
    assert!(
        position(&calls, |call| matches!(call, Call::Initialize { .. }))
            < position(&calls, |call| matches!(call, Call::StorageId { .. })),
        "the instance is initialised before any other call: {calls:?}"
    );
}

#[test]
fn a_storage_request_from_a_timer_during_initialisation_is_answered() {
    let fake = Fake::new(script(|call| match call {
        // As a CDM that waits for its storage id before it says it is initialised.
        Call::Initialize { .. } => Some(vec![Step::Later {
            delay_ms: 1,
            steps: vec![Step::RequestStorageId(0)],
        }]),
        Call::StorageId { .. } => Some(vec![Step::Initialized(true)]),
        _ => None,
    }));
    let runtime = start(&fake, deadlines(|d| d.initialise = HANG_GUARD));
    assert_eq!(runtime.initialise(), Ok(()));
    assert!(runtime.shutdown());
    assert_eq!(
        storage_answers(&fake),
        [Call::StorageId {
            version: 0,
            given: false
        }]
    );
}

#[test]
fn host_calls_of_an_instance_destroyed_first_are_dropped() {
    let mut attempts = 0;
    let fake = Fake::new(script(move |call| match call {
        Call::Initialize { .. } => {
            attempts += 1;
            Some(if attempts == 1 {
                vec![
                    Step::Later {
                        delay_ms: 0,
                        steps: vec![Step::Mark("timer of a failed instance")],
                    },
                    Step::RequestStorageId(1),
                    Step::Initialized(false),
                ]
            } else {
                vec![Step::Initialized(true)]
            })
        }
        _ => None,
    }));
    let runtime = start(&fake, QUICK);
    assert_eq!(runtime.initialise(), Err(Error::InitialisationFailed));
    assert_eq!(runtime.initialise(), Ok(()));
    // Turns enough for the timer, due at once, to fire had it been kept.
    licensed(&runtime);
    assert!(runtime.shutdown());
    assert!(marks(&fake).is_empty());
    assert!(storage_answers(&fake).is_empty());
}

#[test]
fn host_calls_of_a_creation_that_gives_no_instance_are_dropped() {
    let mut creates = 0;
    let fake = Fake::new(script(move |call| match call {
        Call::Create { .. } => {
            creates += 1;
            Some(if creates == 1 {
                vec![
                    Step::Later {
                        delay_ms: 0,
                        steps: vec![Step::Mark("timer from no instance")],
                    },
                    Step::RequestStorageId(1),
                ]
            } else {
                Vec::new()
            })
        }
        _ => None,
    }));
    fake.refuse_create.store(true, Ordering::Relaxed);
    let runtime = start(&fake, QUICK);
    assert_eq!(runtime.initialise(), Err(Error::InterfaceNotSupported(11)));
    fake.refuse_create.store(false, Ordering::Relaxed);
    assert_eq!(runtime.initialise(), Ok(()));
    licensed(&runtime);
    assert!(runtime.shutdown());
    assert!(marks(&fake).is_empty());
    assert!(storage_answers(&fake).is_empty());
}

#[test]
fn an_instance_of_another_interface_is_refused() {
    let fake = Fake::new(well_behaved());
    fake.refuse_create.store(true, Ordering::Relaxed);
    let runtime = start(&fake, QUICK);
    assert_eq!(runtime.initialise(), Err(Error::InterfaceNotSupported(11)));
    assert!(runtime.shutdown());
}

// Opening sessions.

#[test]
fn a_session_gives_its_licence_request_and_is_temporary_with_cenc_init_data() {
    let fake = Fake::new(well_behaved());
    let runtime = ready(&fake);
    let init_data = pssh_for_key(&key(9));
    let (_, challenge) = runtime.open_session(&init_data).unwrap();
    assert_eq!(challenge, b"challenge");
    assert!(runtime.shutdown());
    let calls = fake.calls();
    assert!(calls.iter().any(|call| matches!(call,
        Call::CreateSession { session_type: 0, init_data_type: 0, init_data: data, .. } if *data == init_data)));
}

#[test]
fn the_licence_request_may_come_before_the_session_is_named() {
    let fake = Fake::new(script(|call| match call {
        Call::CreateSession { promise, .. } => Some(vec![
            Step::Message {
                session: b"early".to_vec(),
                kind: MESSAGE_LICENCE_REQUEST,
                message: b"first".to_vec(),
            },
            Step::NewSession {
                promise: *promise,
                session: b"early".to_vec(),
            },
        ]),
        _ => None,
    }));
    let runtime = ready(&fake);
    let (_, challenge) = runtime.open_session(b"init").unwrap();
    assert_eq!(challenge, b"first");
    assert_eq!(ignored(&runtime), 0);
    assert!(runtime.shutdown());
}

#[test]
fn answers_may_come_later_from_the_cdms_timers() {
    let fake = Fake::new(script(|call| match call {
        Call::CreateSession { promise, .. } => Some(vec![
            Step::Later {
                delay_ms: 10,
                steps: vec![Step::Message {
                    session: b"s".to_vec(),
                    kind: MESSAGE_LICENCE_REQUEST,
                    message: b"late".to_vec(),
                }],
            },
            Step::Later {
                delay_ms: 5,
                steps: vec![Step::NewSession {
                    promise: *promise,
                    session: b"s".to_vec(),
                }],
            },
        ]),
        Call::UpdateSession {
            promise,
            session,
            response,
        } => Some(vec![Step::Later {
            delay_ms: 3,
            steps: vec![
                Step::Resolve(*promise),
                Step::Later {
                    delay_ms: 3,
                    steps: vec![Step::Keys {
                        session: session.clone(),
                        keys: vec![(response.clone(), 0)],
                    }],
                },
            ],
        }]),
        _ => None,
    }));
    let runtime = ready(&fake);
    let (session, challenge) = runtime.open_session(b"init").unwrap();
    assert_eq!(challenge, b"late");
    assert_eq!(runtime.give_licence(session, &key(1), &key(1)), Ok(()));
    assert!(runtime.shutdown());
}

#[test]
fn a_refused_init_data_carries_the_cdms_exception_code_and_message() {
    let fake = Fake::new(script(|call| match call {
        Call::CreateSession { promise, .. } => Some(vec![Step::Reject {
            promise: *promise,
            exception: 1,
            system_code: 42,
            message: "unknown pssh".to_owned(),
        }]),
        _ => None,
    }));
    let runtime = ready(&fake);
    let error = runtime.open_session(b"init").unwrap_err();
    assert_eq!(
        error,
        Error::InitDataRefused {
            exception: "NotSupportedError",
            system_code: 42,
            message: "unknown pssh".to_owned()
        }
    );
    assert!(error.to_string().contains("unknown pssh"));
    assert!(runtime.shutdown());
}

#[test]
fn an_individualization_request_is_not_provisioned_and_its_session_closed() {
    let (closed, closing) = channel();
    let closed = Mutex::new(closed);
    let fake = Fake::new(script(move |call| match call {
        Call::CreateSession { promise, .. } => Some(vec![
            Step::NewSession {
                promise: *promise,
                session: b"unprovisioned".to_vec(),
            },
            Step::Message {
                session: b"unprovisioned".to_vec(),
                kind: MESSAGE_INDIVIDUALIZATION_REQUEST,
                message: b"certificate request".to_vec(),
            },
        ]),
        Call::CloseSession { promise, session } => Some(vec![
            Step::Resolve(*promise),
            Step::Closed {
                session: session.clone(),
            },
            Step::Signal(closed.lock().unwrap().clone()),
        ]),
        _ => None,
    }));
    let runtime = ready(&fake);
    assert_eq!(
        runtime.open_session(b"init").map(|_| ()),
        Err(Error::NotProvisioned)
    );
    wait_for(&closing);
    assert!(runtime.shutdown());
    assert!(fake.calls().iter().any(|call| matches!(call,
        Call::CloseSession { session, .. } if session == b"unprovisioned")));
}

#[test]
fn a_session_with_no_licence_request_in_time_is_an_error_and_is_closed() {
    let fake = Fake::new(script(|call| match call {
        Call::CreateSession { promise, .. } => Some(vec![Step::NewSession {
            promise: *promise,
            session: b"mute".to_vec(),
        }]),
        _ => None,
    }));
    let runtime = start(
        &fake,
        deadlines(|d| d.session_request = Duration::from_millis(30)),
    );
    runtime.initialise().unwrap();
    assert_eq!(
        runtime.open_session(b"init").map(|_| ()),
        Err(Error::NoLicenceRequest)
    );
    assert!(runtime.shutdown());
    assert!(
        fake.calls()
            .iter()
            .any(|call| matches!(call, Call::CloseSession { session, .. } if session == b"mute"))
    );
}

#[test]
fn a_session_named_after_its_deadline_is_closed_when_it_is() {
    let (closed, closing) = channel();
    let closed = Mutex::new(closed);
    let fake = Fake::new(script(move |call| match call {
        Call::CreateSession { promise, .. } => Some(vec![Step::Later {
            delay_ms: 60,
            steps: vec![Step::NewSession {
                promise: *promise,
                session: b"late".to_vec(),
            }],
        }]),
        Call::CloseSession { promise, session } => Some(vec![
            Step::Resolve(*promise),
            Step::Closed {
                session: session.clone(),
            },
            Step::Signal(closed.lock().unwrap().clone()),
        ]),
        _ => None,
    }));
    let runtime = start(
        &fake,
        deadlines(|d| d.session_request = Duration::from_millis(20)),
    );
    runtime.initialise().unwrap();
    assert_eq!(
        runtime.open_session(b"init").map(|_| ()),
        Err(Error::NoLicenceRequest)
    );
    wait_for(&closing);
    assert!(runtime.shutdown());
}

// Licences and key statuses.

fn licence_with(
    steps: impl Fn(u32, &[u8], &[u8]) -> Vec<Step> + Send + 'static,
) -> Result<(), Error> {
    let fake = Fake::new(script(move |call| match call {
        Call::UpdateSession {
            promise,
            session,
            response,
        } => Some(steps(*promise, session, response)),
        _ => None,
    }));
    let runtime = start(&fake, deadlines(|d| d.licence = Duration::from_millis(40)));
    runtime.initialise().unwrap();
    let (session, _) = runtime.open_session(b"init").unwrap();
    let result = runtime.give_licence(session, &key(1), &key(1));
    assert!(runtime.shutdown());
    result
}

fn keys(session: &[u8], statuses: &[(u8, u32)]) -> Step {
    let keys = statuses
        .iter()
        .map(|&(id, status)| (key(id).to_vec(), status))
        .collect();
    Step::Keys {
        session: session.to_vec(),
        keys,
    }
}

#[test]
fn key_statuses_may_come_before_the_promise_resolves() {
    assert_eq!(
        licence_with(|promise, session, _| vec![keys(session, &[(1, 0)]), Step::Resolve(promise)]),
        Ok(())
    );
}

#[test]
fn key_statuses_may_come_after_the_promise_resolves() {
    assert_eq!(
        licence_with(|promise, session, _| vec![Step::Resolve(promise), keys(session, &[(1, 0)])]),
        Ok(())
    );
}

#[test]
fn a_key_pending_then_usable_is_accepted() {
    let result = licence_with(|promise, session, _| {
        vec![
            keys(session, &[(1, 5)]),
            Step::Resolve(promise),
            Step::Later {
                delay_ms: 5,
                steps: vec![keys(session, &[(2, 0), (1, 0)])],
            },
        ]
    });
    assert_eq!(result, Ok(()));
}

#[test]
fn a_final_status_other_than_usable_is_named() {
    for (status, name) in [
        (1, "internal error"),
        (2, "expired"),
        (3, "output restricted"),
        (4, "output downscaled"),
        (6, "released"),
    ] {
        let result = licence_with(move |promise, session, _| {
            vec![keys(session, &[(1, status)]), Step::Resolve(promise)]
        });
        assert_eq!(result, Err(Error::KeyNotUsable { status: name }));
    }
}

#[test]
fn a_refused_licence_carries_the_cdms_message_and_system_code() {
    let result = licence_with(|promise, session, _| {
        vec![
            keys(session, &[(1, 5)]),
            Step::Reject {
                promise,
                exception: 2,
                system_code: 7,
                message: "licence too old".to_owned(),
            },
        ]
    });
    let error = result.unwrap_err();
    assert_eq!(
        error,
        Error::LicenceRefused {
            exception: "InvalidStateError",
            system_code: 7,
            message: "licence too old".to_owned()
        }
    );
    assert!(error.to_string().contains("licence too old") && error.to_string().contains('7'));
}

#[test]
fn a_licence_without_the_needed_key_meets_its_deadline() {
    let result =
        licence_with(|promise, session, _| vec![keys(session, &[(2, 0)]), Step::Resolve(promise)]);
    assert_eq!(result, Err(Error::LicenceNotTaken));
    let result =
        licence_with(|promise, session, _| vec![keys(session, &[(1, 5)]), Step::Resolve(promise)]);
    assert_eq!(result, Err(Error::LicenceNotTaken));
    let result = licence_with(|_, session, _| vec![keys(session, &[(1, 0)])]);
    assert_eq!(
        result,
        Err(Error::LicenceNotTaken),
        "the promise never resolved"
    );
}

#[test]
fn answers_for_unknown_promises_are_ignored_and_noted() {
    let fake = Fake::new(script(|call| match call {
        Call::UpdateSession {
            promise,
            session,
            response,
        } => Some(vec![
            Step::Resolve(promise + 100),
            Step::Reject {
                promise: promise + 101,
                exception: 0,
                system_code: 0,
                message: String::new(),
            },
            Step::NewSession {
                promise: promise + 102,
                session: b"stray".to_vec(),
            },
            Step::KeyStatusResolved(*promise),
            Step::NewSession {
                promise: *promise,
                session: b"wrong kind".to_vec(),
            },
            Step::Keys {
                session: session.clone(),
                keys: vec![(response.clone(), 0)],
            },
            Step::Resolve(*promise),
            Step::Resolve(*promise),
        ]),
        _ => None,
    }));
    let runtime = ready(&fake);
    let session = licensed(&runtime);
    let mut data = unmask(b"sample", &[]);
    runtime.decrypt(session, &mut data, &cenc(&key(1))).unwrap();
    assert_eq!(data, b"sample");
    // Read after a later request: the licence is answered before the CDM thread has acted on
    // the last events of its call.
    assert_eq!(ignored(&runtime), 6);
    assert!(runtime.shutdown());
}

#[test]
fn callbacks_for_a_session_already_closed_are_ignored_and_noted() {
    let fake = Fake::new(script(|call| match call {
        Call::CreateSession {
            promise, init_data, ..
        } if init_data == b"second" => {
            let closed = b"session-1".to_vec();
            Some(vec![
                Step::Keys {
                    session: closed.clone(),
                    keys: vec![(key(1).to_vec(), 0)],
                },
                Step::Message {
                    session: closed.clone(),
                    kind: MESSAGE_LICENCE_REQUEST,
                    message: b"x".to_vec(),
                },
                Step::Expiration {
                    session: closed.clone(),
                    time: 1.0,
                },
                Step::Closed { session: closed },
                Step::NewSession {
                    promise: *promise,
                    session: b"second".to_vec(),
                },
                Step::Message {
                    session: b"second".to_vec(),
                    kind: MESSAGE_LICENCE_REQUEST,
                    message: b"two".to_vec(),
                },
            ])
        }
        _ => None,
    }));
    let runtime = ready(&fake);
    let (first, _) = runtime.open_session(b"first").unwrap();
    runtime.close(first);
    let before = ignored(&runtime);
    let (second, challenge) = runtime.open_session(b"second").unwrap();
    assert_eq!(challenge, b"two");
    assert_eq!(ignored(&runtime) - before, 4);
    assert_eq!(
        runtime.decrypt(first, &mut [1], &cenc(&key(1))),
        Err(Error::SessionClosed)
    );
    runtime.give_licence(second, &key(3), &key(3)).unwrap();
    assert!(runtime.shutdown());
}

#[test]
fn a_session_the_cdm_closes_fails_its_licence_and_decrypts() {
    let fake = Fake::new(script(|call| match call {
        Call::UpdateSession { session, .. } => Some(vec![Step::Closed {
            session: session.clone(),
        }]),
        _ => None,
    }));
    let runtime = ready(&fake);
    let (session, _) = runtime.open_session(b"init").unwrap();
    assert_eq!(
        runtime.give_licence(session, &key(1), &key(1)),
        Err(Error::SessionClosed)
    );
    assert_eq!(
        runtime.decrypt(session, &mut [1, 2], &cenc(&key(1))),
        Err(Error::SessionClosed)
    );
    runtime.close(session);
    assert!(runtime.shutdown());
    assert!(
        !fake
            .calls()
            .iter()
            .any(|call| matches!(call, Call::CloseSession { .. })),
        "already closed by the CDM"
    );
}

#[test]
fn a_key_that_expires_later_fails_its_decrypts_with_key_expired() {
    let fake = Fake::new(script(|call| match call {
        Call::Decrypt(_) => Some(vec![keys(b"session-1", &[(1, 2)])]),
        _ => None,
    }));
    let runtime = ready(&fake);
    let session = licensed(&runtime);
    let mut data = unmask(b"first", &[]);
    runtime.decrypt(session, &mut data, &cenc(&key(1))).unwrap();
    assert_eq!(data, b"first");
    assert_eq!(
        runtime.decrypt(session, &mut [1, 2, 3], &cenc(&key(1))),
        Err(Error::KeyExpired)
    );
    assert!(runtime.shutdown());
    assert_eq!(
        decrypts(&fake).len(),
        1,
        "the expired key's decrypt never reached the CDM"
    );
}

#[test]
fn several_sessions_are_open_at_once() {
    let fake = Fake::new(well_behaved());
    let runtime = ready(&fake);
    let sessions: Vec<u64> = (1..=3)
        .map(|byte| {
            let (session, _) = runtime.open_session(&pssh_for_key(&key(byte))).unwrap();
            runtime
                .give_licence(session, &key(byte), &key(byte))
                .unwrap();
            session
        })
        .collect();
    for (byte, &session) in (1..=3).zip(&sessions) {
        let mut data = unmask(b"some sample", &[]);
        runtime
            .decrypt(session, &mut data, &cenc(&key(byte)))
            .unwrap();
        assert_eq!(data, b"some sample");
    }
    runtime.close(sessions[1]);
    let mut data = unmask(b"still open", &[]);
    runtime
        .decrypt(sessions[2], &mut data, &cenc(&key(3)))
        .unwrap();
    assert!(runtime.shutdown());
    let closes: Vec<Vec<u8>> = fake
        .calls()
        .into_iter()
        .filter_map(|call| match call {
            Call::CloseSession { session, .. } => Some(session),
            _ => None,
        })
        .collect();
    assert_eq!(
        closes[0], b"session-2",
        "closed when dropped, the others at shutdown"
    );
    assert_eq!(closes.len(), 3);
}

#[test]
fn each_call_gets_a_fresh_promise_id() {
    let fake = Fake::new(well_behaved());
    let runtime = ready(&fake);
    for byte in 1..=3 {
        let (session, _) = runtime.open_session(b"init").unwrap();
        runtime
            .give_licence(session, &key(byte), &key(byte))
            .unwrap();
        runtime.close(session);
    }
    assert!(runtime.shutdown());
    let mut promises: Vec<u32> = fake
        .calls()
        .into_iter()
        .filter_map(|call| match call {
            Call::CreateSession { promise, .. }
            | Call::UpdateSession { promise, .. }
            | Call::CloseSession { promise, .. } => Some(promise),
            _ => None,
        })
        .collect();
    let count = promises.len();
    promises.sort_unstable();
    promises.dedup();
    assert_eq!((count, promises.len()), (9, 9));
}

// The host's other duties.

#[test]
fn timers_fire_in_order_of_due_time_never_before_and_a_timer_may_set_another() {
    let (done, finished) = channel();
    let done = Mutex::new(done);
    let fake = Fake::new(script(move |call| match call {
        Call::Initialize { .. } => Some(vec![
            Step::Later {
                delay_ms: 30,
                steps: vec![Step::Mark("30")],
            },
            Step::Later {
                delay_ms: 10,
                steps: vec![Step::Mark("10")],
            },
            Step::Later {
                delay_ms: 20,
                steps: vec![
                    Step::Mark("20"),
                    Step::Later {
                        delay_ms: 25,
                        steps: vec![
                            Step::Mark("set by a timer"),
                            Step::Signal(done.lock().unwrap().clone()),
                        ],
                    },
                ],
            },
            Step::Initialized(true),
        ]),
        _ => None,
    }));
    let started = Instant::now();
    let runtime = ready(&fake);
    wait_for(&finished);
    assert!(
        started.elapsed() >= Duration::from_millis(45),
        "no timer fires before its delay"
    );
    assert!(runtime.shutdown());
    let marks: Vec<&str> = fake
        .calls()
        .into_iter()
        .filter_map(|call| match call {
            Call::Mark(name) => Some(name),
            _ => None,
        })
        .collect();
    assert_eq!(marks, ["10", "20", "30", "set by a timer"]);
}

#[test]
fn storage_ids_are_answered_with_none_after_the_call() {
    let (answered, answer) = channel();
    let answered = Mutex::new(answered);
    let fake = Fake::new(script(move |call| match call {
        Call::Initialize { .. } => Some(vec![Step::RequestStorageId(0), Step::Initialized(true)]),
        Call::StorageId { .. } => Some(vec![Step::Signal(answered.lock().unwrap().clone())]),
        _ => None,
    }));
    let runtime = ready(&fake);
    wait_for(&answer);
    assert_eq!(
        runtime
            .shared()
            .counters
            .storage_requests
            .load(Ordering::Relaxed),
        1
    );
    assert!(runtime.shutdown());
    assert!(fake.calls().contains(&Call::StorageId {
        version: 0,
        given: false
    }));
}

#[test]
fn unanswered_requests_and_expiries_change_nothing() {
    let fake = Fake::new(script(|call| match call {
        Call::Initialize { .. } => Some(vec![
            Step::Unanswered(UNANSWERED_QUERY_OUTPUT_PROTECTION),
            Step::Unanswered(UNANSWERED_QUERY_OUTPUT_PROTECTION),
            Step::Unanswered(UNANSWERED_FILE_IO),
            Step::Unanswered(UNANSWERED_METRICS),
            Step::Unanswered(77),
            Step::Initialized(true),
        ]),
        Call::UpdateSession {
            promise,
            session,
            response,
        } => Some(vec![
            Step::Expiration {
                session: session.clone(),
                time: 1.0e9,
            },
            Step::Keys {
                session: session.clone(),
                keys: vec![(response.clone(), 0)],
            },
            Step::Resolve(*promise),
        ]),
        _ => None,
    }));
    let runtime = ready(&fake);
    licensed(&runtime);
    assert_eq!(ignored(&runtime), 0);
    assert!(runtime.shutdown());
}

#[test]
fn the_wall_time_is_seconds_since_the_epoch() {
    // SAFETY: the callback ignores its host pointer.
    let seconds = unsafe { (HOST_CALLS.wall_time)(std::ptr::null_mut()) };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    assert!((seconds - now).abs() < 60.0);
}

#[test]
fn a_callback_from_outside_the_cdm_thread_is_dropped_unread() {
    let fake = Fake::new(well_behaved());
    let runtime = ready(&fake);
    // SAFETY: the callback checks the pointer against this thread's inbox, which it has none
    // of, before reading anything: neither pointer is read.
    unsafe {
        (HOST_CALLS.on_resolve)(std::ptr::dangling_mut(), 1);
        (HOST_CALLS.on_session_closed)(std::ptr::null_mut(), std::ptr::dangling(), 1_000_000);
    }
    licensed(&runtime);
    assert!(runtime.shutdown());
}

#[test]
fn at_rest_the_thread_sleeps() {
    let fake = Fake::new(well_behaved());
    let runtime = ready(&fake);
    let _session = licensed(&runtime);
    let turns = runtime.shared().counters.turns.load(Ordering::Relaxed);
    thread::sleep(Duration::from_millis(200));
    let later = runtime.shared().counters.turns.load(Ordering::Relaxed);
    // The turn that follows the licence's answer may start after the first reading; anything
    // more is a wake-up at rest (polling, even at 100 Hz, would add 20).
    assert!(later - turns <= 1, "{} wake-ups at rest", later - turns);
    assert!(runtime.shutdown());
}

// Decrypting.

#[test]
fn a_decrypt_gives_the_cdm_the_sample_and_writes_the_cleartext_in_place() {
    let fake = Fake::new(well_behaved());
    let runtime = ready(&fake);
    let session = licensed(&runtime);
    let clear: Vec<u8> = (0..32).collect();
    let runs = [(5, 16), (3, 8)];
    let mut data = unmask(&clear, &runs);
    let iv: [u8; 16] = core::array::from_fn(|i| 0x10 + i as u8);
    let sample = Protected {
        key_id: &key(1),
        iv: &iv,
        subsamples: &runs,
        scheme: Scheme::Cenc,
    };
    runtime.decrypt(session, &mut data, &sample).unwrap();
    assert_eq!(data, clear);

    let mut data = unmask(&clear, &[]);
    let sample = Protected {
        key_id: &key(1),
        iv: &iv,
        subsamples: &[],
        scheme: Scheme::Cbcs { crypt: 1, skip: 9 },
    };
    runtime.decrypt(session, &mut data, &sample).unwrap();
    assert_eq!(data, clear);
    assert!(runtime.shutdown());

    let decrypts = decrypts(&fake);
    let expected = DecryptCall {
        size: 32,
        scheme: SCHEME_CENC,
        key_id: key(1).to_vec(),
        iv: iv.to_vec(),
        subsamples: runs.to_vec(),
        crypt: 0,
        skip: 0,
        timestamp: 0,
    };
    assert_eq!(decrypts[0], expected);
    assert_eq!(
        decrypts[1],
        DecryptCall {
            scheme: SCHEME_CBCS,
            subsamples: Vec::new(),
            crypt: 1,
            skip: 9,
            ..expected
        }
    );
    assert_eq!(
        fake.buffers_held.load(Ordering::Relaxed),
        0,
        "every buffer destroyed"
    );
}

#[test]
fn each_cdm_status_is_its_own_error_and_leaves_the_sample_unchanged() {
    let status = Arc::new(Mutex::new(0));
    let chosen = Arc::clone(&status);
    let fake = Fake::new(script(move |call| match call {
        Call::Decrypt(_) => Some(vec![Step::Status(*chosen.lock().unwrap())]),
        _ => None,
    }));
    let runtime = ready(&fake);
    let session = licensed(&runtime);
    let expected = [
        (1, Error::NeedMoreData),
        (2, Error::NoKey),
        (3, Error::DecryptorInitialisation),
        (4, Error::DecryptionFailed),
        (5, Error::DecodeError),
        (6, Error::DeferredInitialisation),
        (7, Error::UnknownStatus(7)),
    ];
    for (value, error) in expected {
        *status.lock().unwrap() = value;
        let mut data = vec![1, 2, 3];
        assert_eq!(
            runtime.decrypt(session, &mut data, &cenc(&key(1))),
            Err(error)
        );
        assert_eq!(data, [1, 2, 3]);
    }
    assert_eq!(
        crate::error::decrypt_status_error(2).unwrap().to_string(),
        "decrypting: no key for this content"
    );
    assert!(runtime.shutdown());
}

#[test]
fn a_cleartext_of_another_length_is_an_error() {
    let fake = Fake::new(script(|call| match call {
        Call::Decrypt(decrypt) => Some(vec![Step::OutputLength(decrypt.size - 1)]),
        _ => None,
    }));
    let runtime = ready(&fake);
    let session = licensed(&runtime);
    let mut data = vec![9; 10];
    assert_eq!(
        runtime.decrypt(session, &mut data, &cenc(&key(1))),
        Err(Error::WrongLength {
            expected: 10,
            got: 9
        })
    );
    assert_eq!(data, [9; 10]);
    assert!(runtime.shutdown());
    assert_eq!(fake.buffers_held.load(Ordering::Relaxed), 0);
}

#[test]
fn subsamples_that_do_not_add_up_are_refused_before_the_cdm() {
    let fake = Fake::new(well_behaved());
    let runtime = ready(&fake);
    let session = licensed(&runtime);
    let mut data = vec![0; 10];
    let refused = |runs: &[(u32, u32)], data: &mut [u8]| {
        let sample = Protected {
            key_id: &key(1),
            iv: &[0; 16],
            subsamples: runs,
            scheme: Scheme::Cenc,
        };
        runtime.decrypt(session, data, &sample)
    };
    assert_eq!(
        refused(&[(5, 6)], &mut data),
        Err(Error::SubsamplesMismatch {
            sample: 10,
            runs: 11
        })
    );
    assert_eq!(
        refused(&[(1, 2), (3, 3)], &mut data),
        Err(Error::SubsamplesMismatch {
            sample: 10,
            runs: 9
        })
    );
    assert_eq!(
        refused(&[(u32::MAX, u32::MAX)], &mut data),
        Err(Error::SubsamplesMismatch {
            sample: 10,
            runs: 2 * u64::from(u32::MAX)
        })
    );
    assert_eq!(
        refused(&[], &mut []),
        Ok(()),
        "an empty sample has nothing to decrypt"
    );
    assert_eq!(refused(&[(0, 0)], &mut []), Ok(()));
    assert!(runtime.shutdown());
    assert!(decrypts(&fake).is_empty());
}

// The queue and shutdown.

/// A session licensed for `key(1)` whose first decrypt waits at the returned gate.
fn with_first_decrypt_held() -> (Arc<Fake>, Runtime, u64, Arc<Gate>) {
    let gate = Arc::new(Gate::default());
    let held = Arc::clone(&gate);
    let mut first = true;
    let fake = Fake::new(script(move |call| match call {
        Call::Decrypt(_) if first => {
            first = false;
            Some(vec![Step::Wait(Arc::clone(&held))])
        }
        _ => None,
    }));
    let runtime = ready(&fake);
    let session = licensed(&runtime);
    (fake, runtime, session, gate)
}

fn queued(runtime: &Runtime) -> usize {
    runtime.shared().lock_queue().len()
}

/// Waits, without a clock, until `count` requests are queued.
fn until_queued(runtime: &Runtime, count: usize) {
    while queued(runtime) < count {
        thread::yield_now();
    }
}

#[test]
fn a_licence_sent_while_decrypts_wait_goes_before_them() {
    let (fake, runtime, session, gate) = with_first_decrypt_held();
    let (second, _) = runtime.open_session(b"next track").unwrap();
    thread::scope(|scope| {
        let runtime = &runtime;
        let first =
            scope.spawn(move || runtime.decrypt(session, &mut unmask(b"a", &[]), &cenc(&key(1))));
        gate.wait_until_entered();
        let waiting: Vec<_> = (0..3)
            .map(|_| {
                scope
                    .spawn(move || runtime.decrypt(session, &mut unmask(b"b", &[]), &cenc(&key(1))))
            })
            .collect();
        until_queued(runtime, 3);
        let licence = scope.spawn(move || runtime.give_licence(second, &key(2), &key(2)));
        until_queued(runtime, 4);
        gate.open();
        assert_eq!(first.join().unwrap(), Ok(()));
        assert_eq!(licence.join().unwrap(), Ok(()));
        for decrypt in waiting {
            assert_eq!(decrypt.join().unwrap(), Ok(()));
        }
    });
    assert!(runtime.shutdown());
    let calls = fake.calls();
    let first_decrypt = position(&calls, |call| matches!(call, Call::Decrypt(_)));
    assert!(
        matches!(calls[first_decrypt + 1], Call::UpdateSession { .. }),
        "{calls:?}"
    );
    assert!(
        calls[first_decrypt + 2..first_decrypt + 5]
            .iter()
            .all(|call| matches!(call, Call::Decrypt(_)))
    );
}

#[test]
fn shutdown_answers_waiting_requests_finishes_the_call_under_way_and_closes_everything() {
    let (fake, runtime, session, gate) = with_first_decrypt_held();
    thread::scope(|scope| {
        let runtime = &runtime;
        let under_way = scope.spawn(move || {
            let mut data = unmask(b"under way", &[]);
            runtime
                .decrypt(session, &mut data, &cenc(&key(1)))
                .map(|()| data)
        });
        gate.wait_until_entered();
        let waiting = scope.spawn(move || runtime.decrypt(session, &mut [1], &cenc(&key(1))));
        let opening = scope.spawn(move || runtime.open_session(b"init").map(|_| ()));
        until_queued(runtime, 2);
        let shutting = scope.spawn(move || runtime.shutdown());
        assert_eq!(waiting.join().unwrap(), Err(Error::ShutDown));
        assert_eq!(opening.join().unwrap(), Err(Error::ShutDown));
        assert_eq!(
            runtime.open_session(b"after").map(|_| ()),
            Err(Error::ShutDown)
        );
        gate.open();
        assert_eq!(under_way.join().unwrap(), Ok(b"under way".to_vec()));
        assert!(shutting.join().unwrap(), "the thread ended in time");
    });
    assert_eq!(
        runtime.decrypt(session, &mut [1], &cenc(&key(1))),
        Err(Error::ShutDown)
    );
    assert!(runtime.shutdown(), "shutting down again does no harm");
    let calls = fake.calls();
    let tail: Vec<&Call> = calls.iter().rev().take(3).rev().collect();
    assert!(
        matches!(
            tail[..],
            [
                Call::CloseSession { .. },
                Call::Destroy,
                Call::ModuleDeinitialised
            ]
        ),
        "{tail:?}"
    );
    assert!(!runtime.shared().counters.panicked.load(Ordering::Relaxed));
}

#[test]
fn shutdown_answers_the_waits_for_the_cdm() {
    let fake = Fake::new(script(|call| match call {
        Call::UpdateSession { .. } => Some(Vec::new()),
        _ => None,
    }));
    let runtime = start(&fake, deadlines(|d| d.licence = Duration::from_secs(60)));
    runtime.initialise().unwrap();
    let (session, _) = runtime.open_session(b"init").unwrap();
    thread::scope(|scope| {
        let runtime = &runtime;
        let licence = scope.spawn(move || runtime.give_licence(session, &key(1), &key(1)));
        while !fake
            .calls()
            .iter()
            .any(|call| matches!(call, Call::UpdateSession { .. }))
        {
            thread::yield_now();
        }
        assert!(runtime.shutdown());
        assert_eq!(licence.join().unwrap(), Err(Error::ShutDown));
    });
}

#[test]
fn shutdown_answers_an_initialisation_under_way() {
    let fake = Fake::new(script(|call| match call {
        Call::Initialize { .. } => Some(Vec::new()),
        _ => None,
    }));
    let runtime = start(&fake, deadlines(|d| d.initialise = Duration::from_secs(60)));
    thread::scope(|scope| {
        let runtime = &runtime;
        let initialising = scope.spawn(move || runtime.initialise());
        while !fake
            .calls()
            .iter()
            .any(|call| matches!(call, Call::Initialize { .. }))
        {
            thread::yield_now();
        }
        assert!(runtime.shutdown());
        assert_eq!(initialising.join().unwrap(), Err(Error::ShutDown));
    });
    assert_eq!(fake.calls().last(), Some(&Call::ModuleDeinitialised));
}

#[test]
fn shutdown_is_bounded_when_the_cdm_never_closes() {
    let fake = Fake::new(script(|call| match call {
        Call::CloseSession { .. } => Some(Vec::new()),
        _ => None,
    }));
    let runtime = start(
        &fake,
        deadlines(|d| {
            d.close = Duration::from_secs(60);
            d.shutdown = Duration::from_millis(200);
        }),
    );
    runtime.initialise().unwrap();
    runtime.open_session(b"init").unwrap();
    assert!(runtime.shutdown(), "the thread ended within the bound");
    let calls = fake.calls();
    assert!(
        calls
            .iter()
            .any(|call| matches!(call, Call::CloseSession { .. }))
    );
    assert_eq!(calls.last(), Some(&Call::ModuleDeinitialised));
}

#[test]
fn shutdown_goes_on_as_soon_as_the_last_close_times_out() {
    let fake = Fake::new(script(|call| match call {
        Call::CloseSession { .. } => Some(Vec::new()),
        _ => None,
    }));
    // Were shutdown to wait out its whole bound, this test would outlast its hang guard.
    let runtime = start(
        &fake,
        deadlines(|d| {
            d.close = Duration::from_millis(10);
            d.shutdown = Duration::from_secs(120);
        }),
    );
    runtime.initialise().unwrap();
    runtime.open_session(b"init").unwrap();
    let (ended, shut) = channel();
    thread::scope(|scope| {
        let runtime = &runtime;
        scope.spawn(move || ended.send(runtime.shutdown()));
        assert_eq!(shut.recv_timeout(HANG_GUARD), Ok(true));
    });
}
