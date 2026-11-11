//! The CDM thread (§3, §4, §7, §8): it owns the CDM instance, makes every call into it,
//! receives every callback, fires the timers and holds the host's state.
//!
//! Each turn of its loop fires the timers due, ends the waits whose deadline passed, makes the
//! calls events left for later, then takes one request, a session's step before any decrypt.
//! With nothing to do it sleeps until the next timer or deadline, or until a request comes.
//!
//! Callbacks only record events ([`crate::boundary`]); the thread acts on them once the call
//! into the CDM has returned, and anything that calls the CDM again waits in `later`.

use std::collections::VecDeque;
use std::ffi::c_void;
use std::fmt;
use std::ptr;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Instant;

use crate::Error;
use crate::Scheme;
use crate::boundary::{
    self, DecryptInput, DecryptOutput, Event, HOST_CALLS, Inbox, Subsample, TimerContext,
};
use crate::error::{decrypt_status_error, exception_name};
use crate::library::Library;
use crate::log;
use crate::runtime::{Reply, Request, SampleRef, SessionKey, Shared};
use crate::timers::Timers;

/// The key system the host asks the library for.
const KEY_SYSTEM: &str = "com.widevine.alpha";

/// How many messages for a session not yet known are kept while a session is opening.
const EARLY_MESSAGE_LIMIT: usize = 8;

/// Part of the shutdown deadline given to closing the sessions; the rest leaves time to destroy
/// the instance and end within it.
const SHUTDOWN_CLOSING_SHARE: u32 = 4;
const SHUTDOWN_CLOSING_OF: u32 = 5;

/// A key's status, as the CDM reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyStatus {
    Usable,
    InternalError,
    Expired,
    OutputRestricted,
    OutputDownscaled,
    Pending,
    Released,
    /// A value interface 11 does not define: treated as not usable yet.
    Unknown(u32),
}

impl KeyStatus {
    fn from_cdm(value: u32) -> KeyStatus {
        match value {
            0 => KeyStatus::Usable,
            1 => KeyStatus::InternalError,
            2 => KeyStatus::Expired,
            3 => KeyStatus::OutputRestricted,
            4 => KeyStatus::OutputDownscaled,
            5 => KeyStatus::Pending,
            6 => KeyStatus::Released,
            other => KeyStatus::Unknown(other),
        }
    }

    /// Whether the status is the CDM's last word on the key for this licence.
    fn is_final(self) -> bool {
        !matches!(self, KeyStatus::Pending | KeyStatus::Unknown(_))
    }

    fn name(self) -> &'static str {
        match self {
            KeyStatus::Usable => "usable",
            KeyStatus::InternalError => "internal error",
            KeyStatus::Expired => "expired",
            KeyStatus::OutputRestricted => "output restricted",
            KeyStatus::OutputDownscaled => "output downscaled",
            KeyStatus::Pending => "pending",
            KeyStatus::Released => "released",
            KeyStatus::Unknown(_) => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionState {
    /// Waiting for its id or its licence request.
    Opening,
    Open,
    /// `CloseSession` was called; `closed` once the CDM said so.
    Closing {
        closed: bool,
    },
    /// The CDM closed it on its own; its handle has not been dropped yet.
    Closed,
}

struct SessionEntry {
    key: SessionKey,
    /// The CDM's id for it, once its promise resolved.
    cdm_id: Option<Vec<u8>>,
    state: SessionState,
    keys: Vec<(Vec<u8>, KeyStatus)>,
}

impl SessionEntry {
    /// Why a request to this session, which is not open, is refused: the CDM closed it on its
    /// own, or the host closed it.
    fn closed_error(&self) -> Error {
        if self.state == SessionState::Closed {
            Error::SessionClosed
        } else {
            Error::SessionClosedByHost
        }
    }

    fn key_status(&self, key_id: &[u8]) -> Option<KeyStatus> {
        self.keys
            .iter()
            .find(|(id, _)| id == key_id)
            .map(|&(_, status)| status)
    }
}

/// A caller, or the host itself, waiting for the CDM to answer through a callback.
enum Wait {
    Initialise {
        reply: Reply<()>,
        deadline: Instant,
    },
    Open {
        promise: u32,
        session: SessionKey,
        challenge: Option<Vec<u8>>,
        reply: Reply<Vec<u8>>,
        deadline: Instant,
    },
    /// `promise` is `None` once it resolved; the key is then awaited.
    Licence {
        promise: Option<u32>,
        session: SessionKey,
        key_id: [u8; 16],
        reply: Reply<()>,
        deadline: Instant,
    },
    /// `promise` is `None` once it resolved.
    Close {
        promise: Option<u32>,
        session: SessionKey,
        deadline: Instant,
    },
    /// A session's opening that failed before the CDM named the session: if it does later, the
    /// session is closed.
    AbandonedOpen {
        promise: u32,
        deadline: Instant,
    },
}

impl Wait {
    fn deadline(&self) -> Instant {
        match self {
            Wait::Initialise { deadline, .. }
            | Wait::Open { deadline, .. }
            | Wait::Licence { deadline, .. }
            | Wait::Close { deadline, .. }
            | Wait::AbandonedOpen { deadline, .. } => *deadline,
        }
    }

    fn promise(&self) -> Option<u32> {
        match self {
            Wait::Initialise { .. } => None,
            Wait::Open { promise, .. } | Wait::AbandonedOpen { promise, .. } => Some(*promise),
            Wait::Licence { promise, .. } | Wait::Close { promise, .. } => *promise,
        }
    }
}

/// A call into the CDM an event asked for, made once the current call has returned.
enum Later {
    AnswerStorageId { version: u32 },
    Close { session: SessionKey },
    DestroyInstance,
}

/// A message for a session the CDM had not named yet.
struct EarlyMessage {
    session: Vec<u8>,
    kind: u32,
    message: Vec<u8>,
}

struct Instance {
    cdm: *mut c_void,
    ready: bool,
}

/// What the turn found to do once the timers and deadlines were seen to.
enum Next {
    Request(Request),
    ShutDown,
    Turn,
}

/// The state of the CDM thread.
pub(crate) struct Host {
    shared: Arc<Shared>,
    library: Library,
    inbox: Box<Inbox>,
    instance: Option<Instance>,
    timers: Timers,
    sessions: Vec<SessionEntry>,
    waits: Vec<Wait>,
    early_messages: Vec<EarlyMessage>,
    later: VecDeque<Later>,
    last_promise: u32,
    /// The subsample runs of the decrypt under way, laid out for the CDM; reused.
    subsamples: Vec<Subsample>,
    /// The unanswered calls already logged, one bit each.
    unanswered_logged: u32,
}

impl Host {
    /// The host of the calling thread, which must be the CDM thread: the inbox is its own.
    pub(crate) fn new(shared: Arc<Shared>, library: Library) -> Host {
        Host {
            shared,
            library,
            inbox: Inbox::for_this_thread(),
            instance: None,
            timers: Timers::default(),
            sessions: Vec::new(),
            waits: Vec::new(),
            early_messages: Vec::new(),
            later: VecDeque::new(),
            last_promise: 0,
            subsamples: Vec::new(),
            unanswered_logged: 0,
        }
    }

    /// Runs until shutdown.
    pub(crate) fn run(&mut self) {
        self.library.module.initialize();
        let version = self.library.module.version();
        self.log(format_args!("the CDM's version is {version}"));
        loop {
            self.shared.counters.turns.fetch_add(1, Ordering::Relaxed);
            self.fire_due_timers();
            self.end_waits_past_deadline(Instant::now());
            // Before any request: a failed instance is destroyed before the next one is asked for.
            self.make_later_calls();
            match self.next() {
                Next::Request(request) => self.handle(request),
                Next::ShutDown => break,
                Next::Turn => {}
            }
        }
        self.shut_down();
    }

    /// Takes the next request, or sleeps until there is something to do.
    fn next(&mut self) -> Next {
        let wake_at = self.next_wake();
        let mut queue = self.shared.lock_queue();
        if queue.shutdown_started().is_some() {
            return Next::ShutDown;
        }
        // While the CDM initialises, the host makes no other call (the header's rule).
        if !self.initialising()
            && let Some(request) = queue.pop()
        {
            return Next::Request(request);
        }
        if !self.later.is_empty() || self.inbox.has_events() {
            return Next::Turn;
        }
        match wake_at {
            // The guard the wait gives back is dropped at once: the turn starts again.
            None => drop(self.shared.wake.wait(queue)),
            Some(at) => {
                let now = Instant::now();
                if at > now {
                    drop(self.shared.wake.wait_timeout(queue, at - now));
                }
            }
        }
        Next::Turn
    }

    /// When the next timer or deadline is due.
    fn next_wake(&self) -> Option<Instant> {
        let deadlines = self.waits.iter().map(Wait::deadline);
        deadlines.chain(self.timers.next_due()).min()
    }

    fn initialising(&self) -> bool {
        self.waits
            .iter()
            .any(|wait| matches!(wait, Wait::Initialise { .. }))
    }

    fn ready_cdm(&self) -> Option<*mut c_void> {
        self.instance
            .as_ref()
            .filter(|instance| instance.ready)
            .map(|instance| instance.cdm)
    }

    /// The instance, while it is ready or initialising: its timers fire and its storage-id
    /// requests are answered. A failed instance, waiting to be destroyed, is not called.
    fn live_cdm(&self) -> Option<*mut c_void> {
        let initialising = self.initialising();
        self.instance
            .as_ref()
            .filter(|instance| instance.ready || initialising)
            .map(|instance| instance.cdm)
    }

    fn handle(&mut self, request: Request) {
        match request {
            Request::Initialise { reply } => self.initialise(reply),
            Request::OpenSession {
                session,
                init_data,
                reply,
            } => {
                self.open_session(session, &init_data, reply);
            }
            Request::GiveLicence {
                session,
                licence,
                key_id,
                reply,
            } => {
                self.give_licence(session, &licence, key_id, reply);
            }
            Request::Decrypt {
                session,
                sample,
                reply,
            } => {
                let result = self.decrypt(session, &sample);
                // The caller's memory is not used after this answer.
                self.shared.answer(reply, result);
            }
            Request::Close { session } => {
                self.close(session, Instant::now() + self.shared.deadlines.close)
            }
        }
    }

    // Requests.

    fn initialise(&mut self, reply: Reply<()>) {
        if let Some(instance) = &self.instance {
            let result = if instance.ready {
                Ok(())
            } else {
                Err(Error::NotInitialised)
            };
            self.shared.answer(reply, result);
            return;
        }
        let calls = self.library.calls;
        // SAFETY: `entry` is what this table's `create` expects (the library's
        // `CreateCdmInstance`, or a fake's state); the key system is `KEY_SYSTEM.len()` bytes;
        // the inbox outlives every instance, as the host destroys them before it ends.
        let cdm = unsafe {
            (calls.create)(
                self.library.entry.0,
                KEY_SYSTEM.as_ptr().cast(),
                KEY_SYSTEM.len() as u32,
                self.inbox.as_host(),
                &HOST_CALLS,
            )
        };
        if cdm.is_null() {
            // What the CDM asked for while giving no instance is dropped: there is none to answer.
            self.act_on_events();
            self.shared.answer(
                reply,
                Err(Error::InterfaceNotSupported(calls.interface_version)),
            );
            return;
        }
        // Known before its creation's events are acted on: the CDM may have set timers or asked
        // for a storage id inside `CreateCdmInstance`.
        self.instance = Some(Instance { cdm, ready: false });
        self.act_on_events();
        let deadline = Instant::now() + self.shared.deadlines.initialise;
        self.waits.push(Wait::Initialise { reply, deadline });
        // SAFETY: `cdm` was just created and is not destroyed.
        unsafe { (calls.initialize)(cdm, false, false, false) };
        self.act_on_events();
    }

    fn open_session(&mut self, session: SessionKey, init_data: &[u8], reply: Reply<Vec<u8>>) {
        let Some(cdm) = self.ready_cdm() else {
            self.shared.answer(reply, Err(Error::NotInitialised));
            return;
        };
        let promise = self.new_promise();
        self.sessions.push(SessionEntry {
            key: session,
            cdm_id: None,
            state: SessionState::Opening,
            keys: Vec::new(),
        });
        let deadline = Instant::now() + self.shared.deadlines.session_request;
        self.waits.push(Wait::Open {
            promise,
            session,
            challenge: None,
            reply,
            deadline,
        });
        // SAFETY: `cdm` is the ready instance; the init data's length fits (checked by the
        // caller's side) and the bytes live for the call.
        unsafe {
            (self.library.calls.create_session)(
                cdm,
                promise,
                boundary::SESSION_TEMPORARY,
                boundary::INIT_DATA_CENC,
                init_data.as_ptr(),
                init_data.len() as u32,
            );
        }
        self.act_on_events();
    }

    fn give_licence(
        &mut self,
        session: SessionKey,
        licence: &[u8],
        key_id: [u8; 16],
        reply: Reply<()>,
    ) {
        let Some(cdm) = self.ready_cdm() else {
            self.shared.answer(reply, Err(Error::NotInitialised));
            return;
        };
        let cdm_id = match self.session(session) {
            Some(entry) if entry.state == SessionState::Open => {
                entry.cdm_id.clone().unwrap_or_default()
            }
            other => {
                let error = other.map_or(Error::SessionClosedByHost, SessionEntry::closed_error);
                self.shared.answer(reply, Err(error));
                return;
            }
        };
        let promise = self.new_promise();
        let deadline = Instant::now() + self.shared.deadlines.licence;
        self.waits.push(Wait::Licence {
            promise: Some(promise),
            session,
            key_id,
            reply,
            deadline,
        });
        // SAFETY: `cdm` is the ready instance; the session id and the licence live for the
        // call, and their lengths fit (the CDM gave the id; the caller's side checked the
        // licence).
        unsafe {
            (self.library.calls.update_session)(
                cdm,
                promise,
                cdm_id.as_ptr().cast(),
                cdm_id.len() as u32,
                licence.as_ptr(),
                licence.len() as u32,
            );
        }
        self.act_on_events();
    }

    fn decrypt(&mut self, session: SessionKey, sample: &SampleRef) -> Result<(), Error> {
        let cdm = self.ready_cdm().ok_or(Error::NotInitialised)?;
        let entry = self.session(session).ok_or(Error::SessionClosedByHost)?;
        if entry.state != SessionState::Open {
            return Err(entry.closed_error());
        }
        if entry.key_status(&sample.key_id) == Some(KeyStatus::Expired) {
            return Err(Error::KeyExpired);
        }
        self.subsamples.clear();
        // SAFETY: the request holding `sample` is answered only after this function returns.
        let runs = unsafe { sample.subsamples() };
        self.subsamples
            .extend(runs.iter().map(|&(clear_bytes, cipher_bytes)| Subsample {
                clear_bytes,
                cipher_bytes,
            }));
        let (scheme, crypt, skip) = match sample.scheme {
            Scheme::Cenc => (boundary::SCHEME_CENC, 0, 0),
            Scheme::Cbcs { crypt, skip } => {
                (boundary::SCHEME_CBCS, u32::from(crypt), u32::from(skip))
            }
        };
        let input = DecryptInput {
            data: sample.data,
            data_size: sample.len as u32,
            scheme,
            key_id: sample.key_id.as_ptr(),
            key_id_size: 16,
            iv: sample.iv.as_ptr(),
            iv_size: 16,
            subsamples: if self.subsamples.is_empty() {
                ptr::null()
            } else {
                self.subsamples.as_ptr()
            },
            subsample_count: self.subsamples.len() as u32,
            crypt_byte_block: crypt,
            skip_byte_block: skip,
            // Only comes back in the decrypted block, which the host does not read.
            timestamp: 0,
        };
        let mut output = DecryptOutput {
            data: ptr::null(),
            size: 0,
            buffer: ptr::null_mut(),
        };
        let calls = self.library.calls;
        // SAFETY: `cdm` is the ready instance; every pointer in `input` is valid for the call:
        // the caller's sample (its length checked to fit), the key id, the IV and the runs.
        let status = unsafe { (calls.decrypt)(cdm, &input, &mut output) };
        let result = match decrypt_status_error(status) {
            Some(error) => Err(error),
            None => copy_cleartext(&output, sample),
        };
        if status == 0 && !output.buffer.is_null() {
            // SAFETY: a successful decrypt hands its buffer to the host, which destroys it once.
            unsafe { (calls.release_buffer)(output.buffer) };
        }
        self.act_on_events();
        result
    }

    /// Closes the session; `deadline` bounds the wait for the CDM.
    fn close(&mut self, session: SessionKey, deadline: Instant) {
        let Some(index) = self.session_index(session) else {
            return;
        };
        let entry = &self.sessions[index];
        match (entry.state, entry.cdm_id.clone(), self.ready_cdm()) {
            (SessionState::Closing { .. }, _, _) => {}
            (SessionState::Opening | SessionState::Open, Some(cdm_id), Some(cdm)) => {
                self.sessions[index].state = SessionState::Closing { closed: false };
                let promise = self.new_promise();
                self.waits.push(Wait::Close {
                    promise: Some(promise),
                    session,
                    deadline,
                });
                // SAFETY: `cdm` is the ready instance; the id lives for the call and its
                // length fits, as the CDM gave it.
                unsafe {
                    (self.library.calls.close_session)(
                        cdm,
                        promise,
                        cdm_id.as_ptr().cast(),
                        cdm_id.len() as u32,
                    );
                }
                self.act_on_events();
            }
            // Closed by the CDM, never named, or no instance: nothing left to close.
            _ => {
                self.sessions.remove(index);
            }
        }
    }

    // Timers and calls made later.

    fn fire_due_timers(&mut self) {
        let round = self.timers.round(Instant::now());
        // Checked before each timer: one may end the initialisation in failure.
        while let Some(cdm) = self.live_cdm() {
            let Some(context) = self.timers.pop_due(round) else {
                break;
            };
            self.shared
                .counters
                .timers_fired
                .fetch_add(1, Ordering::Relaxed);
            // SAFETY: the instance set this timer and is not destroyed (destroying clears the
            // timers); the context is handed back as it was given.
            unsafe { (self.library.calls.timer_expired)(cdm, context.0) };
            self.act_on_events();
        }
    }

    /// Makes the calls events asked for; those they lead to wait for the next turn.
    fn make_later_calls(&mut self) {
        for _ in 0..self.later.len() {
            let Some(call) = self.later.pop_front() else {
                break;
            };
            match call {
                Later::AnswerStorageId { version } => {
                    // Answered while the instance initialises too: a CDM may wait for its
                    // storage id before it says it is initialised.
                    if let Some(cdm) = self.live_cdm() {
                        // SAFETY: `cdm` is the live instance; no storage id is given.
                        unsafe { (self.library.calls.storage_id)(cdm, version, ptr::null(), 0) };
                        self.act_on_events();
                    }
                }
                Later::Close { session } => {
                    self.close(session, Instant::now() + self.shared.deadlines.close);
                }
                Later::DestroyInstance => self.destroy_instance(),
            }
        }
    }

    fn destroy_instance(&mut self) {
        let Some(instance) = self.instance.take() else {
            return;
        };
        // What it asked for goes with it.
        self.timers.clear();
        self.later
            .retain(|call| !matches!(call, Later::AnswerStorageId { .. }));
        // SAFETY: the instance is destroyed once: it was just taken out of the host.
        unsafe { (self.library.calls.destroy)(instance.cdm) };
        self.act_on_events();
    }

    // Deadlines.

    fn end_waits_past_deadline(&mut self, now: Instant) {
        while let Some(index) = self.waits.iter().position(|wait| wait.deadline() <= now) {
            match self.waits.remove(index) {
                Wait::Initialise { reply, .. } => {
                    self.log(format_args!("initialising the CDM: no answer in time"));
                    self.later.push_back(Later::DestroyInstance);
                    if let Some(instance) = &mut self.instance {
                        instance.ready = false;
                    }
                    self.shared
                        .answer(reply, Err(Error::InitialisationTimedOut));
                }
                Wait::Open {
                    promise,
                    session,
                    reply,
                    ..
                } => {
                    self.log(format_args!(
                        "opening a session: no licence request in time"
                    ));
                    self.shared.answer(reply, Err(Error::NoLicenceRequest));
                    self.abandon_opening(promise, session, false);
                }
                Wait::Licence { reply, .. } => {
                    self.log(format_args!("taking a licence: no usable key in time"));
                    self.shared.answer(reply, Err(Error::LicenceNotTaken));
                }
                Wait::Close { session, .. } => {
                    self.log(format_args!(
                        "closing a session: the CDM did not close it in time"
                    ));
                    self.forget_session(session);
                }
                Wait::AbandonedOpen { .. } => {}
            }
        }
    }

    // Events.

    /// Acts on the events the last call into the CDM recorded, in order.
    fn act_on_events(&mut self) {
        loop {
            let events = self.inbox.take();
            if events.is_empty() {
                break;
            }
            for event in events {
                self.act_on(event);
            }
        }
        if !self.early_messages.is_empty() && !self.opening_unnamed() {
            for _ in self.early_messages.drain(..) {
                self.shared.counters.ignored.fetch_add(1, Ordering::Relaxed);
            }
            self.log(format_args!(
                "messages for sessions never named were ignored"
            ));
        }
    }

    fn act_on(&mut self, event: Event) {
        match event {
            Event::TimerSet { delay_ms, context } => self.set_timer(delay_ms, context),
            Event::Initialized { success } => self.initialised(success),
            Event::KeyStatusResolved => self.ignore("a key status for a promise never made"),
            Event::NewSessionResolved { promise, session } => self.session_named(promise, session),
            Event::Resolved { promise } => self.resolved(promise),
            Event::Rejected {
                promise,
                exception,
                system_code,
                message,
            } => {
                self.rejected(promise, exception, system_code, message);
            }
            Event::Message {
                session,
                kind,
                message,
            } => self.message(session, kind, message),
            Event::KeysChanged { session, keys } => self.keys_changed(&session, keys),
            Event::ExpirationChanged { session, expiry } => match self.known_session(&session) {
                // 0 means "no such time exists" or "never expires" (the CDM's header). A licence
                // that gave a deadline and then 0 would log two lines a song, the second no use
                // to read: 0 is not logged.
                Some(_) if expiry == 0.0 => {}
                Some(_) => self.log(format_args!(
                    "a session's licence expires at {expiry} s since 1970"
                )),
                None => self.ignore("an expiry for an unknown session"),
            },
            Event::Closed { session } => self.session_closed(&session),
            Event::StorageIdRequested { version } => {
                self.shared
                    .counters
                    .storage_requests
                    .fetch_add(1, Ordering::Relaxed);
                if self.instance.is_some() {
                    self.later.push_back(Later::AnswerStorageId { version });
                } else {
                    self.ignore("a storage-id request from no instance");
                }
            }
            Event::Unanswered { request } => self.unanswered(request),
        }
    }

    fn set_timer(&mut self, delay_ms: i64, context: TimerContext) {
        if self.instance.is_none() {
            self.ignore("a timer from no instance");
            return;
        }
        self.shared
            .counters
            .timers_set
            .fetch_add(1, Ordering::Relaxed);
        if !self.timers.set(Instant::now(), delay_ms, context) {
            self.ignore("a timer too far away to ever fire");
        }
    }

    fn initialised(&mut self, success: bool) {
        let Some(index) = self
            .waits
            .iter()
            .position(|wait| matches!(wait, Wait::Initialise { .. }))
        else {
            self.ignore("an initialisation nobody waits for");
            return;
        };
        let Wait::Initialise { reply, .. } = self.waits.remove(index) else {
            return;
        };
        if success {
            if let Some(instance) = &mut self.instance {
                instance.ready = true;
            }
            self.shared.answer(reply, Ok(()));
        } else {
            self.log(format_args!("initialising the CDM: it failed"));
            self.later.push_back(Later::DestroyInstance);
            self.shared.answer(reply, Err(Error::InitialisationFailed));
        }
    }

    fn session_named(&mut self, promise: u32, cdm_id: Vec<u8>) {
        let Some(index) = self.wait_for_promise(promise) else {
            self.ignore("a session for an unknown promise");
            return;
        };
        match &self.waits[index] {
            Wait::Open { session, .. } => {
                let session = *session;
                if cdm_id.is_empty() || self.known_session(&cdm_id).is_some() {
                    let Wait::Open { reply, .. } = self.waits.remove(index) else {
                        return;
                    };
                    self.forget_session(session);
                    let error = Error::InitDataRefused {
                        exception: "no exception",
                        system_code: 0,
                        message: "the CDM gave no usable session id".to_owned(),
                    };
                    self.shared.answer(reply, Err(error));
                    return;
                }
                if let Some(entry) = self.session_mut(session) {
                    entry.cdm_id = Some(cdm_id.clone());
                }
                let early: Vec<EarlyMessage> = {
                    let (theirs, others) = std::mem::take(&mut self.early_messages)
                        .into_iter()
                        .partition(|message| message.session == cdm_id);
                    self.early_messages = others;
                    theirs
                };
                for message in early {
                    self.message(message.session, message.kind, message.message);
                }
                self.settle_opening(session);
            }
            Wait::AbandonedOpen { .. } => {
                self.waits.remove(index);
                let session = self.shared.new_session_key();
                self.sessions.push(SessionEntry {
                    key: session,
                    cdm_id: Some(cdm_id),
                    state: SessionState::Opening,
                    keys: Vec::new(),
                });
                self.later.push_back(Later::Close { session });
            }
            _ => self.ignore("a session for a promise of another kind"),
        }
    }

    fn resolved(&mut self, promise: u32) {
        let Some(index) = self.wait_for_promise(promise) else {
            self.ignore("an answer for an unknown promise");
            return;
        };
        match &mut self.waits[index] {
            Wait::Licence {
                promise, session, ..
            } => {
                *promise = None;
                let session = *session;
                self.settle_licences(session);
            }
            Wait::Close {
                promise, session, ..
            } => {
                *promise = None;
                let session = *session;
                self.settle_closing(session);
            }
            _ => self.ignore("an answer of the wrong kind for a promise"),
        }
    }

    fn rejected(&mut self, promise: u32, exception: u32, system_code: u32, message: String) {
        let Some(index) = self.wait_for_promise(promise) else {
            self.ignore("a rejection for an unknown promise");
            return;
        };
        let exception = exception_name(exception);
        match self.waits.remove(index) {
            Wait::Open {
                promise,
                session,
                reply,
                ..
            } => {
                self.log(format_args!(
                    "opening a session: the CDM refused the init data"
                ));
                self.shared.answer(
                    reply,
                    Err(Error::InitDataRefused {
                        exception,
                        system_code,
                        message,
                    }),
                );
                self.abandon_opening(promise, session, true);
            }
            Wait::Licence { reply, .. } => {
                self.log(format_args!("taking a licence: the CDM refused it"));
                self.shared.answer(
                    reply,
                    Err(Error::LicenceRefused {
                        exception,
                        system_code,
                        message,
                    }),
                );
            }
            Wait::Close { session, .. } => {
                self.log(format_args!(
                    "closing a session: the CDM refused, {message}"
                ));
                self.forget_session(session);
            }
            Wait::AbandonedOpen { .. } => {}
            Wait::Initialise { .. } => {}
        }
    }

    fn message(&mut self, cdm_id: Vec<u8>, kind: u32, message: Vec<u8>) {
        let Some(index) = self.known_session(&cdm_id) else {
            if self.opening_unnamed() {
                if self.early_messages.len() == EARLY_MESSAGE_LIMIT {
                    self.early_messages.remove(0);
                    self.ignore("a message for an unknown session");
                }
                self.early_messages.push(EarlyMessage {
                    session: cdm_id,
                    kind,
                    message,
                });
            } else {
                self.ignore("a message for an unknown session");
            }
            return;
        };
        let session = self.sessions[index].key;
        match self.sessions[index].state {
            SessionState::Closed | SessionState::Closing { .. } => {
                self.ignore("a message for a closed session");
                return;
            }
            SessionState::Opening | SessionState::Open => {}
        }
        match kind {
            boundary::MESSAGE_LICENCE_REQUEST => {
                let state = self.sessions[index].state;
                self.licence_request(session, state, message);
            }
            boundary::MESSAGE_INDIVIDUALIZATION_REQUEST => {
                self.log(format_args!(
                    "the CDM asks for a device certificate: not provisioned, nothing is sent"
                ));
                self.fail_session(session, &Error::NotProvisioned);
            }
            boundary::MESSAGE_LICENCE_RENEWAL => {
                self.log(format_args!(
                    "the CDM asks for a licence renewal; it is not sent"
                ));
            }
            boundary::MESSAGE_LICENCE_RELEASE => {
                self.log(format_args!(
                    "the CDM asks for a licence release; it is not sent"
                ));
            }
            _ => self.ignore("a message of an unknown type"),
        }
    }

    /// Keeps the first licence request of a session that is opening as its challenge; any other
    /// is logged and never sent.
    fn licence_request(&mut self, session: SessionKey, state: SessionState, message: Vec<u8>) {
        let challenge = self.waits.iter_mut().find_map(|wait| match wait {
            Wait::Open {
                session: waiting,
                challenge,
                ..
            } if *waiting == session => Some(challenge),
            _ => None,
        });
        match challenge {
            Some(challenge) if challenge.is_none() => {
                *challenge = Some(message);
                self.settle_opening(session);
            }
            // A session still opening that nobody waits for has failed: its close is under way.
            None if state == SessionState::Opening => self.log(format_args!(
                "the CDM made a licence request after its session's opening had failed or \
                 timed out; it is not sent, and the session is closed"
            )),
            _ => self.log(format_args!(
                "the CDM made another licence request; it is not sent"
            )),
        }
    }

    fn keys_changed(&mut self, cdm_id: &[u8], keys: Vec<(Vec<u8>, u32)>) {
        let Some(index) = self.known_session(cdm_id) else {
            self.ignore("key statuses for an unknown session");
            return;
        };
        let entry = &mut self.sessions[index];
        if matches!(
            entry.state,
            SessionState::Closed | SessionState::Closing { .. }
        ) {
            self.ignore("key statuses for a closed session");
            return;
        }
        entry.keys = keys
            .into_iter()
            .map(|(id, status)| (id, KeyStatus::from_cdm(status)))
            .collect();
        let session = entry.key;
        self.settle_licences(session);
    }

    fn session_closed(&mut self, cdm_id: &[u8]) {
        let Some(index) = self.known_session(cdm_id) else {
            self.ignore("a close for an unknown session");
            return;
        };
        let entry = &mut self.sessions[index];
        let session = entry.key;
        match entry.state {
            SessionState::Closing { closed: false } => {
                entry.state = SessionState::Closing { closed: true };
                self.settle_closing(session);
            }
            SessionState::Closing { closed: true } | SessionState::Closed => {
                self.ignore("a close for a closed session");
            }
            SessionState::Opening | SessionState::Open => {
                entry.state = SessionState::Closed;
                self.log(format_args!("the CDM closed a session"));
                self.fail_session(session, &Error::SessionClosed);
            }
        }
    }

    fn unanswered(&mut self, request: u32) {
        let what = match request {
            boundary::UNANSWERED_QUERY_OUTPUT_PROTECTION => "the output protection's status",
            boundary::UNANSWERED_ENABLE_OUTPUT_PROTECTION => "output protection",
            boundary::UNANSWERED_PLATFORM_CHALLENGE => "a platform challenge",
            boundary::UNANSWERED_FILE_IO => "a file",
            // Deferred decoder initialisation and metrics concern decoders, which are not used.
            _ => return,
        };
        let bit = 1 << request;
        if self.unanswered_logged & bit == 0 {
            self.unanswered_logged |= bit;
            self.log(format_args!("the CDM asked for {what}; not answered"));
        }
    }

    // Settling waits.

    /// Answers the session's opening once it has both its id and its licence request.
    fn settle_opening(&mut self, session: SessionKey) {
        let named = self
            .session(session)
            .is_some_and(|entry| entry.cdm_id.is_some());
        let Some(index) = self.waits.iter().position(|wait| {
            matches!(wait, Wait::Open { session: waiting, challenge: Some(_), .. } if *waiting == session)
        }) else {
            return;
        };
        if !named {
            return;
        }
        let Wait::Open {
            challenge: Some(challenge),
            reply,
            ..
        } = self.waits.remove(index)
        else {
            return;
        };
        if let Some(entry) = self.session_mut(session) {
            entry.state = SessionState::Open;
        }
        self.shared.answer(reply, Ok(challenge));
    }

    /// Answers the session's licences whose outcome is known: resolved with the key usable, or
    /// resolved with a final status other than usable.
    fn settle_licences(&mut self, session: SessionKey) {
        let mut index = 0;
        while index < self.waits.len() {
            let outcome = match &self.waits[index] {
                Wait::Licence {
                    promise,
                    session: waiting,
                    key_id,
                    ..
                } if *waiting == session => self.licence_outcome(*promise, session, key_id),
                _ => None,
            };
            match outcome {
                Some(result) => {
                    if let Wait::Licence { reply, .. } = self.waits.remove(index) {
                        self.shared.answer(reply, result);
                    }
                }
                None => index += 1,
            }
        }
    }

    fn licence_outcome(
        &self,
        promise: Option<u32>,
        session: SessionKey,
        key_id: &[u8; 16],
    ) -> Option<Result<(), Error>> {
        let entry = self.session(session)?;
        if entry.state != SessionState::Open {
            return Some(Err(entry.closed_error()));
        }
        if promise.is_some() {
            return None;
        }
        match entry.key_status(key_id)? {
            KeyStatus::Usable => Some(Ok(())),
            status if status.is_final() => Some(Err(Error::KeyNotUsable {
                status: status.name(),
            })),
            _ => None,
        }
    }

    /// Ends a closing once its promise resolved and the CDM said the session is closed.
    fn settle_closing(&mut self, session: SessionKey) {
        let closed = self
            .session(session)
            .is_some_and(|entry| entry.state == SessionState::Closing { closed: true });
        let Some(index) = self.waits.iter().position(|wait| {
            matches!(wait, Wait::Close { promise: None, session: waiting, .. } if *waiting == session)
        }) else {
            return;
        };
        if closed {
            self.waits.remove(index);
            self.forget_session(session);
            self.log(format_args!("a session closed"));
        }
    }

    /// Fails what waits on the session, its opening or its licences, with `error`.
    fn fail_session(&mut self, session: SessionKey, error: &Error) {
        let mut index = 0;
        while index < self.waits.len() {
            let theirs = match &self.waits[index] {
                Wait::Open {
                    session: waiting, ..
                }
                | Wait::Licence {
                    session: waiting, ..
                } => *waiting == session,
                _ => false,
            };
            if !theirs {
                index += 1;
                continue;
            }
            match self.waits.remove(index) {
                Wait::Open { promise, reply, .. } => {
                    self.shared.answer(reply, Err(error.clone()));
                    self.abandon_opening(promise, session, true);
                }
                Wait::Licence { reply, .. } => self.shared.answer(reply, Err(error.clone())),
                _ => {}
            }
        }
    }

    /// Cleans up after an opening that failed: the session is closed if the CDM named it, and
    /// will be if the CDM names it later (its promise not settled).
    fn abandon_opening(&mut self, promise: u32, session: SessionKey, promise_settled: bool) {
        let Some(entry) = self.session(session) else {
            return;
        };
        match (entry.cdm_id.is_some(), entry.state) {
            (true, SessionState::Opening | SessionState::Open) => {
                self.later.push_back(Later::Close { session });
            }
            (true, _) => self.forget_session(session),
            (false, _) => {
                self.forget_session(session);
                if !promise_settled {
                    let deadline = Instant::now() + self.shared.deadlines.close;
                    self.waits.push(Wait::AbandonedOpen { promise, deadline });
                }
            }
        }
    }

    // Lookups.

    fn session(&self, session: SessionKey) -> Option<&SessionEntry> {
        self.sessions.iter().find(|entry| entry.key == session)
    }

    fn session_mut(&mut self, session: SessionKey) -> Option<&mut SessionEntry> {
        self.sessions.iter_mut().find(|entry| entry.key == session)
    }

    fn session_index(&self, session: SessionKey) -> Option<usize> {
        self.sessions.iter().position(|entry| entry.key == session)
    }

    /// The session the CDM calls `cdm_id`.
    fn known_session(&self, cdm_id: &[u8]) -> Option<usize> {
        self.sessions
            .iter()
            .position(|entry| entry.cdm_id.as_deref() == Some(cdm_id))
    }

    fn forget_session(&mut self, session: SessionKey) {
        self.sessions.retain(|entry| entry.key != session);
    }

    fn wait_for_promise(&self, promise: u32) -> Option<usize> {
        self.waits
            .iter()
            .position(|wait| wait.promise() == Some(promise))
    }

    /// Whether a session is opening that the CDM has not named yet.
    fn opening_unnamed(&self) -> bool {
        self.waits.iter().any(|wait| match wait {
            Wait::Open { session, .. } => self
                .session(*session)
                .is_some_and(|entry| entry.cdm_id.is_none()),
            _ => false,
        })
    }

    /// A promise id no wait uses.
    fn new_promise(&mut self) -> u32 {
        loop {
            self.last_promise = self.last_promise.wrapping_add(1);
            if self.wait_for_promise(self.last_promise).is_none() {
                return self.last_promise;
            }
        }
    }

    /// Logs one line; in tests, also keeps it where the test can read it.
    fn log(&self, what: fmt::Arguments<'_>) {
        log::line(what);
        #[cfg(test)]
        self.shared
            .lines
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(what.to_string());
    }

    fn ignore(&self, what: &str) {
        self.shared.counters.ignored.fetch_add(1, Ordering::Relaxed);
        self.log(format_args!("ignored {what}"));
    }

    // Shutdown.

    /// Answers every wait, closes the sessions still open, destroys the instance and ends the
    /// module, within the shutdown deadline.
    fn shut_down(&mut self) {
        let started = self
            .shared
            .lock_queue()
            .shutdown_started()
            .unwrap_or_else(Instant::now);
        let closing_until =
            started + self.shared.deadlines.shutdown * SHUTDOWN_CLOSING_SHARE / SHUTDOWN_CLOSING_OF;
        for wait in std::mem::take(&mut self.waits) {
            match wait {
                Wait::Initialise { reply, .. } | Wait::Licence { reply, .. } => {
                    self.shared.answer(reply, Err(Error::ShutDown));
                }
                Wait::Open { reply, session, .. } => {
                    self.shared.answer(reply, Err(Error::ShutDown));
                    if self
                        .session(session)
                        .is_some_and(|entry| entry.cdm_id.is_none())
                    {
                        self.forget_session(session);
                    }
                }
                Wait::Close { .. } => self.waits.push(wait),
                Wait::AbandonedOpen { .. } => {}
            }
        }
        self.later.clear();
        let open: Vec<SessionKey> = self
            .sessions
            .iter()
            .filter(|entry| matches!(entry.state, SessionState::Opening | SessionState::Open))
            .map(|entry| entry.key)
            .collect();
        // Each close within its own deadline, and all of them within the shutdown's.
        let deadline = closing_until.min(Instant::now() + self.shared.deadlines.close);
        for session in open {
            self.close(session, deadline);
        }
        self.wait_for_closes(closing_until);
        if self
            .waits
            .iter()
            .any(|wait| matches!(wait, Wait::Close { .. }))
        {
            self.log(format_args!(
                "shutting down: some sessions did not close in time"
            ));
        }
        self.later.clear();
        self.destroy_instance();
        self.library.module.deinitialize();
        let counters = &self.shared.counters;
        self.log(format_args!(
            "shut down; the CDM set {} timers ({} fired) and asked {} times for a storage id; {} callbacks ignored",
            counters.timers_set.load(Ordering::Relaxed),
            counters.timers_fired.load(Ordering::Relaxed),
            counters.storage_requests.load(Ordering::Relaxed),
            counters.ignored.load(Ordering::Relaxed),
        ));
    }

    /// Fires timers until every closing has ended or `until` has come, sleeping in between.
    fn wait_for_closes(&mut self, until: Instant) {
        loop {
            self.shared.counters.turns.fetch_add(1, Ordering::Relaxed);
            self.fire_due_timers();
            self.end_waits_past_deadline(Instant::now());
            // Checked after the timers and deadlines, which may have ended the last closing.
            let closing = self
                .waits
                .iter()
                .any(|wait| matches!(wait, Wait::Close { .. }));
            let now = Instant::now();
            if !closing || now >= until {
                return;
            }
            let wake_at = self.next_wake().map_or(until, |at| at.min(until));
            if wake_at > now {
                std::thread::sleep(wake_at - now);
            }
        }
    }
}

/// Copies a successful decrypt's cleartext over the caller's sample, when it has the sample's
/// length.
fn copy_cleartext(output: &DecryptOutput, sample: &SampleRef) -> Result<(), Error> {
    let got = output.size as usize;
    if got != sample.len || output.data.is_null() {
        let got = if output.data.is_null() { 0 } else { got };
        return Err(Error::WrongLength {
            expected: sample.len,
            got,
        });
    }
    // SAFETY: the CDM's buffer holds `got` = `sample.len` bytes until the host destroys it; the
    // caller's sample is `sample.len` writable bytes while its request is unanswered. `copy`
    // allows the two to overlap, though a CDM writing into its own buffer never does.
    unsafe { ptr::copy(output.data, sample.data, sample.len) };
    Ok(())
}

const _: () = assert!(SHUTDOWN_CLOSING_SHARE < SHUTDOWN_CLOSING_OF);
