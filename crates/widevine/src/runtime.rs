//! The requests callers send to the CDM thread, and how they wait for the answer (§7, §8).
//!
//! Callers on any thread put a request in a queue by priority and wait for its answer; the CDM
//! thread ([`crate::host`]) takes them one at a time. A session's opening, licence and closing
//! go before decrypts. Shutting down answers every request still queued, refuses every later
//! one, and waits for the CDM thread to end, five seconds at most. Should the CDM thread end
//! otherwise, by a panic, the requests queued and every later one get [`Error::HostStopped`].

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{SyncSender, sync_channel};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use crate::Error;
use crate::host::Host;
use crate::library::Library;
use crate::{Protected, Scheme};

/// The host's own name for a session; the CDM's name for it is only known on the CDM thread.
pub(crate) type SessionKey = u64;

/// How long each wait for the CDM may last (§4) and how long shutting down may take (§8).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Deadlines {
    pub initialise: Duration,
    pub session_request: Duration,
    pub licence: Duration,
    pub close: Duration,
    pub shutdown: Duration,
}

impl Deadlines {
    /// The deadlines of the specification.
    pub(crate) const SPECIFIED: Deadlines = Deadlines {
        initialise: Duration::from_secs(10),
        session_request: Duration::from_secs(10),
        licence: Duration::from_secs(10),
        close: Duration::from_secs(5),
        shutdown: Duration::from_secs(5),
    };
}

/// A request for the CDM thread.
pub(crate) enum Request {
    Initialise {
        reply: Reply<()>,
    },
    OpenSession {
        session: SessionKey,
        init_data: Vec<u8>,
        reply: Reply<Vec<u8>>,
    },
    GiveLicence {
        session: SessionKey,
        licence: Vec<u8>,
        key_id: [u8; 16],
        reply: Reply<()>,
    },
    Decrypt {
        session: SessionKey,
        sample: SampleRef,
        reply: Reply<()>,
    },
    Close {
        session: SessionKey,
    },
}

impl Request {
    fn is_decrypt(&self) -> bool {
        matches!(self, Request::Decrypt { .. })
    }

    /// Answers the request with `error`, without the CDM.
    fn refuse(self, shared: &Shared, error: Error) {
        match self {
            Request::Initialise { reply } | Request::GiveLicence { reply, .. } => {
                shared.answer(reply, Err(error));
            }
            Request::OpenSession { reply, .. } => shared.answer(reply, Err(error)),
            Request::Decrypt { reply, .. } => shared.answer(reply, Err(error)),
            // The CDM thread closes every session still open when it shuts down.
            Request::Close { .. } => {}
        }
    }
}

/// Where a request's answer goes: its caller, waiting.
pub(crate) struct Reply<T>(SyncSender<Result<T, Error>>);

/// A sample to decrypt, in the caller's memory.
///
/// The caller waits until the request is answered or dropped, so that the memory stays valid
/// and untouched by anyone else for as long as the request exists.
pub(crate) struct SampleRef {
    pub data: *mut u8,
    pub len: usize,
    pub key_id: [u8; 16],
    pub iv: [u8; 16],
    pub subsamples: *const (u32, u32),
    pub subsample_count: usize,
    pub scheme: Scheme,
}

// SAFETY: the pointers are to the memory of a caller that waits until the request is answered or
// dropped (`Runtime::ask`); meanwhile only the CDM thread uses them, and never afterwards.
unsafe impl Send for SampleRef {}

impl SampleRef {
    /// The subsample runs.
    ///
    /// # Safety
    ///
    /// The request holding this sample has not been answered yet.
    pub(crate) unsafe fn subsamples(&self) -> &[(u32, u32)] {
        if self.subsample_count == 0 {
            return &[];
        }
        // SAFETY: the caller's slice, alive while it waits for the answer.
        unsafe { std::slice::from_raw_parts(self.subsamples, self.subsample_count) }
    }
}

/// The requests waiting, by priority.
#[derive(Default)]
pub(crate) struct Queue {
    control: VecDeque<Request>,
    decrypts: VecDeque<Request>,
    /// When shutting down began.
    shutdown_started: Option<Instant>,
    /// Whether the CDM thread has ended: nobody takes requests any more.
    thread_ended: bool,
}

impl Queue {
    fn push(&mut self, request: Request) {
        if request.is_decrypt() {
            self.decrypts.push_back(request);
        } else {
            self.control.push_back(request);
        }
    }

    /// The next request: a session's opening, licence or closing before any decrypt.
    pub(crate) fn pop(&mut self) -> Option<Request> {
        self.control
            .pop_front()
            .or_else(|| self.decrypts.pop_front())
    }

    pub(crate) fn shutdown_started(&self) -> Option<Instant> {
        self.shutdown_started
    }

    /// Why a new request is refused, if it is: shut down, or the CDM thread ended.
    fn refusal(&self) -> Option<Error> {
        if self.shutdown_started.is_some() {
            Some(Error::ShutDown)
        } else if self.thread_ended {
            Some(Error::HostStopped)
        } else {
            None
        }
    }

    /// Every request waiting, taken out of the queue.
    fn take_all(&mut self) -> Vec<Request> {
        self.control
            .drain(..)
            .chain(self.decrypts.drain(..))
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.control.len() + self.decrypts.len()
    }
}

/// What callers and the CDM thread share.
pub(crate) struct Shared {
    pub queue: Mutex<Queue>,
    /// Wakes the CDM thread: a request came, or shutting down began.
    pub wake: Condvar,
    ended: Mutex<bool>,
    ended_signal: Condvar,
    next_session: AtomicU64,
    pub deadlines: Deadlines,
    pub counters: Counters,
}

/// What the host counts, for diagnostics and tests.
#[derive(Default)]
pub(crate) struct Counters {
    /// Turns of the CDM thread's loop.
    pub turns: AtomicU64,
    /// Requests queued that expect an answer.
    pub accepted: AtomicU64,
    /// Answers sent.
    pub answered: AtomicU64,
    /// Callbacks ignored because they named something unknown.
    pub ignored: AtomicU64,
    pub timers_set: AtomicU64,
    pub timers_fired: AtomicU64,
    pub storage_requests: AtomicU64,
    /// Whether the CDM thread ended by a panic.
    pub panicked: AtomicBool,
}

impl Shared {
    pub(crate) fn lock_queue(&self) -> MutexGuard<'_, Queue> {
        self.queue.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Sends `result` to the caller waiting on `reply`, and counts it.
    pub(crate) fn answer<T>(&self, reply: Reply<T>, result: Result<T, Error>) {
        self.counters.answered.fetch_add(1, Ordering::Relaxed);
        // The caller waits for this answer; it is gone only if its thread is.
        let _ = reply.0.send(result);
    }

    pub(crate) fn new_session_key(&self) -> SessionKey {
        self.next_session.fetch_add(1, Ordering::Relaxed)
    }

    fn mark_ended(&self) {
        *self.ended.lock().unwrap_or_else(PoisonError::into_inner) = true;
        self.ended_signal.notify_all();
    }

    /// Waits until the CDM thread has ended or `until` has come; whether it ended.
    fn wait_ended(&self, until: Instant) -> bool {
        let mut ended = self.ended.lock().unwrap_or_else(PoisonError::into_inner);
        while !*ended {
            let now = Instant::now();
            if now >= until {
                return false;
            }
            ended = self
                .ended_signal
                .wait_timeout(ended, until - now)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
        true
    }
}

/// Marks the CDM thread ended when it returns or unwinds, and answers what nobody will take.
///
/// After a shutdown the queue is empty and requests are refused as shut down already; after a
/// panic, the requests queued, and every later one, get [`Error::HostStopped`]. The requests
/// the thread held themselves (the one under way, the waits for the CDM) are dropped as it
/// unwinds, which their callers see as [`Error::HostStopped`] too.
struct EndOfThread(Arc<Shared>);

impl Drop for EndOfThread {
    fn drop(&mut self) {
        let shared = &self.0;
        if thread::panicking() {
            shared.counters.panicked.store(true, Ordering::Relaxed);
            crate::log::line(format_args!("the CDM thread ended unexpectedly"));
        }
        let waiting = {
            let mut queue = shared.lock_queue();
            queue.thread_ended = true;
            queue.take_all()
        };
        for request in waiting {
            request.refuse(shared, Error::HostStopped);
        }
        shared.mark_ended();
    }
}

/// A handle to a running CDM thread; cheap to clone.
#[derive(Clone)]
pub(crate) struct Runtime {
    shared: Arc<Shared>,
}

impl Runtime {
    /// Starts the thread named `cdm`, which owns `library` from now on.
    ///
    /// # Errors
    ///
    /// [`Error::ThreadNotStarted`] when the system cannot start a thread.
    pub(crate) fn start(library: Library, deadlines: Deadlines) -> Result<Runtime, Error> {
        let shared = Arc::new(Shared {
            queue: Mutex::new(Queue::default()),
            wake: Condvar::new(),
            ended: Mutex::new(false),
            ended_signal: Condvar::new(),
            next_session: AtomicU64::new(1),
            deadlines,
            counters: Counters::default(),
        });
        let for_thread = Arc::clone(&shared);
        thread::Builder::new()
            .name("cdm".to_owned())
            .spawn(move || {
                let _end = EndOfThread(Arc::clone(&for_thread));
                Host::new(for_thread, library).run();
            })
            .map_err(|error| Error::ThreadNotStarted(error.to_string()))?;
        Ok(Runtime { shared })
    }

    #[cfg(test)]
    pub(crate) fn shared(&self) -> &Shared {
        &self.shared
    }

    /// Creates and initialises the CDM instance, or returns at once when it is initialised.
    pub(crate) fn initialise(&self) -> Result<(), Error> {
        self.ask(|reply| Request::Initialise { reply })
    }

    /// Opens a session with `init_data`; its key and its licence request.
    pub(crate) fn open_session(&self, init_data: &[u8]) -> Result<(SessionKey, Vec<u8>), Error> {
        check_size("init data", init_data.len())?;
        let session = self.shared.new_session_key();
        let init_data = init_data.to_vec();
        let challenge = self.ask(|reply| Request::OpenSession {
            session,
            init_data,
            reply,
        })?;
        Ok((session, challenge))
    }

    /// Gives `licence` to the session; answers once `key_id` is usable.
    pub(crate) fn give_licence(
        &self,
        session: SessionKey,
        licence: &[u8],
        key_id: &[u8; 16],
    ) -> Result<(), Error> {
        check_size("a licence", licence.len())?;
        let licence = licence.to_vec();
        let key_id = *key_id;
        self.ask(|reply| Request::GiveLicence {
            session,
            licence,
            key_id,
            reply,
        })
    }

    /// Decrypts one sample in place, refusing before the CDM what it could not take.
    pub(crate) fn decrypt(
        &self,
        session: SessionKey,
        data: &mut [u8],
        sample: &Protected<'_>,
    ) -> Result<(), Error> {
        check_size("a sample", data.len())?;
        check_size("the subsample runs", sample.subsamples.len())?;
        if !sample.subsamples.is_empty() {
            let runs: u64 = sample
                .subsamples
                .iter()
                .map(|&(clear, protected)| u64::from(clear) + u64::from(protected))
                .sum();
            if runs != data.len() as u64 {
                return Err(Error::SubsamplesMismatch {
                    sample: data.len(),
                    runs,
                });
            }
        }
        if data.is_empty() {
            return Ok(());
        }
        let sample = SampleRef {
            data: data.as_mut_ptr(),
            len: data.len(),
            key_id: *sample.key_id,
            iv: *sample.iv,
            subsamples: sample.subsamples.as_ptr(),
            subsample_count: sample.subsamples.len(),
            scheme: sample.scheme,
        };
        self.ask(|reply| Request::Decrypt {
            session,
            sample,
            reply,
        })
    }

    /// Asks for the session to be closed, without waiting: how it went is logged.
    pub(crate) fn close(&self, session: SessionKey) {
        let mut queue = self.shared.lock_queue();
        if queue.refusal().is_none() {
            queue.push(Request::Close { session });
            drop(queue);
            self.shared.wake.notify_one();
        }
    }

    /// Shuts the CDM thread down (§8): every request queued or sent from now on gets
    /// [`Error::ShutDown`], the thread finishes the call under way, closes the sessions, destroys
    /// the instance and ends. Returns whether it ended within the shutdown deadline.
    pub(crate) fn shutdown(&self) -> bool {
        let started = Instant::now();
        let waiting: Vec<Request> = {
            let mut queue = self.shared.lock_queue();
            queue.shutdown_started.get_or_insert(started);
            queue.take_all()
        };
        self.shared.wake.notify_all();
        for request in waiting {
            request.refuse(&self.shared, Error::ShutDown);
        }
        self.shared
            .wait_ended(started + self.shared.deadlines.shutdown)
    }

    /// Queues the request `make` builds and waits for its answer.
    ///
    /// The wait ends only when the request is answered or dropped: a decrypt's request points
    /// into the caller's memory, which must outlive every use the CDM thread makes of it.
    fn ask<T>(&self, make: impl FnOnce(Reply<T>) -> Request) -> Result<T, Error> {
        let (sender, receiver) = sync_channel(1);
        let request = make(Reply(sender));
        {
            let mut queue = self.shared.lock_queue();
            if let Some(error) = queue.refusal() {
                return Err(error);
            }
            queue.push(request);
            self.shared
                .counters
                .accepted
                .fetch_add(1, Ordering::Relaxed);
        }
        self.shared.wake.notify_one();
        // A closed channel means the request was dropped unanswered: the CDM thread ended while
        // it held it.
        receiver.recv().unwrap_or(Err(Error::HostStopped))
    }
}

/// Refuses what does not fit the CDM's 32-bit sizes.
fn check_size(what: &'static str, size: usize) -> Result<(), Error> {
    if u32::try_from(size).is_err() {
        return Err(Error::TooLarge { what, size });
    }
    Ok(())
}
