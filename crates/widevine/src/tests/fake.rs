//! A fake CDM written in Rust, standing where the C++ layer and the real CDM stand: it fills the
//! same [`CdmCalls`] table and calls the host back through the same [`HostCalls`], so that the
//! host runs unchanged against it.
//!
//! A test gives it a script: for each call the host makes, the steps the fake takes, during
//! the call or later from a timer it sets on the host, as a CDM answers.

use std::collections::HashMap;
use std::ffi::{c_char, c_void};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};

use crate::boundary::{CdmCalls, DecryptInput, DecryptOutput, HostCalls, KeyInfo};
use crate::library::{Entry, Library, Module};

/// A call the host made on the fake, as the fake saw it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Call {
    ModuleInitialised,
    ModuleDeinitialised,
    Create {
        key_system: Vec<u8>,
    },
    Initialize {
        distinctive_identifier: bool,
        persistent_state: bool,
        hw_secure_codecs: bool,
    },
    CreateSession {
        promise: u32,
        session_type: u32,
        init_data_type: u32,
        init_data: Vec<u8>,
    },
    UpdateSession {
        promise: u32,
        session: Vec<u8>,
        response: Vec<u8>,
    },
    CloseSession {
        promise: u32,
        session: Vec<u8>,
    },
    Decrypt(DecryptCall),
    StorageId {
        version: u32,
        given: bool,
    },
    Destroy,
    /// A [`Step::Mark`] the fake reached.
    Mark(&'static str),
}

/// What a decrypt gave the fake.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DecryptCall {
    pub size: usize,
    pub scheme: u32,
    pub key_id: Vec<u8>,
    pub iv: Vec<u8>,
    pub subsamples: Vec<(u32, u32)>,
    pub crypt: u32,
    pub skip: u32,
    pub timestamp: i64,
}

/// What the fake does, in order.
#[derive(Debug, Clone)]
pub(crate) enum Step {
    Initialized(bool),
    NewSession {
        promise: u32,
        session: Vec<u8>,
    },
    Resolve(u32),
    Reject {
        promise: u32,
        exception: u32,
        system_code: u32,
        message: String,
    },
    KeyStatusResolved(u32),
    Message {
        session: Vec<u8>,
        kind: u32,
        message: Vec<u8>,
    },
    Keys {
        session: Vec<u8>,
        keys: Vec<(Vec<u8>, u32)>,
    },
    Expiration {
        session: Vec<u8>,
        time: f64,
    },
    Closed {
        session: Vec<u8>,
    },
    RequestStorageId(u32),
    Unanswered(u32),
    /// Sets a timer on the host, and takes the steps when it fires.
    Later {
        delay_ms: i64,
        steps: Vec<Step>,
    },
    /// Records [`Call::Mark`].
    Mark(&'static str),
    /// A decrypt's status; 0 when no step says.
    Status(u32),
    /// A successful decrypt's cleartext length; the sample's when no step says.
    OutputLength(usize),
    /// A decrypt waits for the gate to open before answering.
    Wait(Arc<Gate>),
    /// Tells the test the fake got this far.
    Signal(Sender<()>),
}

/// A door a decrypt waits at, to hold the CDM thread inside a call.
#[derive(Debug, Default)]
pub(crate) struct Gate {
    state: Mutex<(bool, bool)>,
    changed: Condvar,
}

impl Gate {
    fn lock(&self) -> MutexGuard<'_, (bool, bool)> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Waits until a decrypt is inside, waiting at the gate.
    pub(crate) fn wait_until_entered(&self) {
        let mut state = self.lock();
        while !state.0 {
            state = self
                .changed
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }

    pub(crate) fn open(&self) {
        self.lock().1 = true;
        self.changed.notify_all();
    }

    fn pass(&self) {
        let mut state = self.lock();
        state.0 = true;
        self.changed.notify_all();
        while !state.1 {
            state = self
                .changed
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }
}

type Script = Box<dyn FnMut(&Call) -> Vec<Step> + Send>;

/// One fake CDM library: its script, what it was asked, and its knobs.
pub(crate) struct Fake {
    script: Mutex<Script>,
    calls: Mutex<Vec<Call>>,
    /// `create` gives no instance, as a CDM without the interface would.
    pub refuse_create: AtomicBool,
    /// Decrypts are not recorded (for measuring).
    pub quiet_decrypts: AtomicBool,
    /// Buffers handed to the host and not yet released.
    pub buffers_held: AtomicI64,
}

/// The mask the fake "decrypts" protected bytes with.
pub(crate) const MASK: u8 = 0x5a;

impl Fake {
    pub(crate) fn new(script: impl FnMut(&Call) -> Vec<Step> + Send + 'static) -> Arc<Fake> {
        Arc::new(Fake {
            script: Mutex::new(Box::new(script)),
            calls: Mutex::new(Vec::new()),
            refuse_create: AtomicBool::new(false),
            quiet_decrypts: AtomicBool::new(false),
            buffers_held: AtomicI64::new(0),
        })
    }

    /// The library the host loads: this fake's table, with the fake as `create`'s entry.
    pub(crate) fn library(self: &Arc<Self>) -> Library {
        Library {
            calls: &FAKE_CALLS,
            entry: Entry(Arc::as_ptr(self).cast_mut().cast()),
            // The module keeps the fake alive as long as the library, so the entry stays valid.
            module: Box::new(FakeModule(Arc::clone(self))),
        }
    }

    pub(crate) fn calls(&self) -> Vec<Call> {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn record(&self, call: Call) {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(call);
    }

    /// Records the call and asks the script what to do.
    fn react(&self, call: Call) -> Vec<Step> {
        let quiet = matches!(call, Call::Decrypt(_)) && self.quiet_decrypts.load(Ordering::Relaxed);
        let steps = (self.script.lock().unwrap_or_else(PoisonError::into_inner))(&call);
        if !quiet {
            self.record(call);
        }
        steps
    }
}

struct FakeModule(Arc<Fake>);

impl Module for FakeModule {
    fn initialize(&mut self) {
        self.0.record(Call::ModuleInitialised);
    }

    fn deinitialize(&mut self) {
        self.0.record(Call::ModuleDeinitialised);
    }

    fn version(&mut self) -> String {
        "0.0.0.fake".to_owned()
    }
}

/// One instance: the host it calls back, and the steps waiting for its timers.
struct FakeInstance {
    fake: Arc<Fake>,
    host: *mut c_void,
    host_calls: *const HostCalls,
    later: HashMap<usize, Vec<Step>>,
    next_timer: usize,
}

/// What a decrypt's steps decided.
struct DecryptPlan {
    status: u32,
    length: Option<usize>,
}

impl FakeInstance {
    fn host_calls(&self) -> &HostCalls {
        // SAFETY: the host gave a pointer to its static table.
        unsafe { &*self.host_calls }
    }

    /// Takes the steps, calling the host back as a CDM would.
    fn run(&mut self, steps: Vec<Step>) -> DecryptPlan {
        let mut plan = DecryptPlan {
            status: 0,
            length: None,
        };
        for step in steps {
            self.take(step, &mut plan);
        }
        plan
    }

    fn take(&mut self, step: Step, plan: &mut DecryptPlan) {
        match step {
            Step::Later { delay_ms, steps } => {
                self.next_timer += 1;
                let token = self.next_timer;
                self.later.insert(token, steps);
                // SAFETY: `host` is the host's own pointer and the CDM thread is calling the
                // fake; the context is a token handed back, never read.
                unsafe { (self.host_calls().set_timer)(self.host, delay_ms, token as *mut c_void) };
            }
            Step::Mark(name) => self.fake.record(Call::Mark(name)),
            Step::Status(status) => plan.status = status,
            Step::OutputLength(length) => plan.length = Some(length),
            Step::Wait(gate) => gate.pass(),
            // The test may have stopped listening; the fake goes on.
            Step::Signal(sender) => drop(sender.send(())),
            callback => self.call_back(callback),
        }
    }

    /// Calls the host back as the step says.
    fn call_back(&self, step: Step) {
        let host = self.host;
        let calls = self.host_calls();
        let keys: Vec<KeyInfo> = match &step {
            Step::Keys { keys, .. } => keys
                .iter()
                .map(|(id, status)| KeyInfo {
                    key_id: id.as_ptr(),
                    key_id_size: size(id),
                    status: *status,
                    system_code: 0,
                })
                .collect(),
            _ => Vec::new(),
        };
        // SAFETY: `host` is the host's own pointer, and the CDM thread is calling the fake, as
        // every callback requires; each pointer with a size points to that many bytes of
        // `step` or `keys`, which outlive the call.
        unsafe {
            match &step {
                Step::Initialized(success) => (calls.on_initialized)(host, *success),
                Step::NewSession { promise, session } => {
                    (calls.on_resolve_new_session)(host, *promise, chars(session), size(session));
                }
                Step::Resolve(promise) => (calls.on_resolve)(host, *promise),
                Step::Reject {
                    promise,
                    exception,
                    system_code,
                    message,
                } => {
                    let message = message.as_bytes();
                    (calls.on_reject)(
                        host,
                        *promise,
                        *exception,
                        *system_code,
                        chars(message),
                        size(message),
                    );
                }
                Step::KeyStatusResolved(promise) => {
                    (calls.on_resolve_key_status)(host, *promise, 0)
                }
                Step::Message {
                    session,
                    kind,
                    message,
                } => {
                    (calls.on_session_message)(
                        host,
                        chars(session),
                        size(session),
                        *kind,
                        chars(message),
                        size(message),
                    );
                }
                Step::Keys { session, .. } => {
                    (calls.on_keys_change)(
                        host,
                        chars(session),
                        size(session),
                        true,
                        keys.as_ptr(),
                        keys.len() as u32,
                    );
                }
                Step::Expiration { session, time } => {
                    (calls.on_expiration_change)(host, chars(session), size(session), *time);
                }
                Step::Closed { session } => {
                    (calls.on_session_closed)(host, chars(session), size(session))
                }
                Step::RequestStorageId(version) => (calls.request_storage_id)(host, *version),
                Step::Unanswered(request) => (calls.unanswered)(host, *request),
                Step::Later { .. }
                | Step::Mark(_)
                | Step::Status(_)
                | Step::OutputLength(_)
                | Step::Wait(_)
                | Step::Signal(_) => {}
            }
        }
    }
}

fn chars(bytes: &[u8]) -> *const c_char {
    bytes.as_ptr().cast()
}

fn size(bytes: &[u8]) -> u32 {
    bytes.len() as u32
}

/// The fake's table, as the C++ layer's for interface 11.
pub(crate) static FAKE_CALLS: CdmCalls = CdmCalls {
    size: size_of::<CdmCalls>() as u32,
    interface_version: 11,
    create: fake_create,
    initialize: fake_initialize,
    create_session: fake_create_session,
    update_session: fake_update_session,
    close_session: fake_close_session,
    timer_expired: fake_timer_expired,
    decrypt: fake_decrypt,
    release_buffer: fake_release_buffer,
    storage_id: fake_storage_id,
    destroy: fake_destroy,
};

/// # Safety
///
/// `cdm` is an instance `fake_create` returned and `fake_destroy` has not freed, used by one
/// thread at a time.
unsafe fn instance<'a>(cdm: *mut c_void) -> &'a mut FakeInstance {
    // SAFETY: the caller's contract.
    unsafe { &mut *cdm.cast::<FakeInstance>() }
}

/// # Safety
///
/// `data` points to `size` readable bytes, or is null.
unsafe fn read(data: *const u8, size: u32) -> Vec<u8> {
    if data.is_null() || size == 0 {
        return Vec::new();
    }
    // SAFETY: the caller's contract.
    unsafe { std::slice::from_raw_parts(data, size as usize) }.to_vec()
}

/// The fake's [`CdmCalls::create`]; its safety contract is [`CdmCalls`]'s.
unsafe extern "C" fn fake_create(
    entry: *mut c_void,
    key_system: *const c_char,
    key_system_size: u32,
    host: *mut c_void,
    host_calls: *const HostCalls,
) -> *mut c_void {
    let entry = entry.cast_const().cast::<Fake>();
    // SAFETY: `entry` is `Fake::library`'s pointer to a fake its module keeps alive.
    let fake = unsafe {
        Arc::increment_strong_count(entry);
        Arc::from_raw(entry)
    };
    // SAFETY: the host passes its key system's bytes.
    let key_system = unsafe { read(key_system.cast(), key_system_size) };
    let steps = fake.react(Call::Create { key_system });
    let mut instance = FakeInstance {
        fake,
        host,
        host_calls,
        later: HashMap::new(),
        next_timer: 0,
    };
    // A CDM may call its host from inside `CreateCdmInstance`, even one that then gives no
    // instance.
    instance.run(steps);
    if instance.fake.refuse_create.load(Ordering::Relaxed) {
        return std::ptr::null_mut();
    }
    Box::into_raw(Box::new(instance)).cast()
}

/// The fake's [`CdmCalls::initialize`]; its safety contract is [`CdmCalls`]'s.
unsafe extern "C" fn fake_initialize(
    cdm: *mut c_void,
    distinctive: bool,
    persistent: bool,
    hw: bool,
) {
    // SAFETY: the table's contract: a live instance, on the CDM thread.
    let instance = unsafe { instance(cdm) };
    let steps = instance.fake.react(Call::Initialize {
        distinctive_identifier: distinctive,
        persistent_state: persistent,
        hw_secure_codecs: hw,
    });
    instance.run(steps);
}

/// The fake's [`CdmCalls::create_session`]; its safety contract is [`CdmCalls`]'s.
unsafe extern "C" fn fake_create_session(
    cdm: *mut c_void,
    promise: u32,
    session_type: u32,
    init_data_type: u32,
    init_data: *const u8,
    init_data_size: u32,
) {
    // SAFETY: the table's contract: a live instance, on the CDM thread, and the init data's
    // bytes.
    let (instance, init_data) = unsafe { (instance(cdm), read(init_data, init_data_size)) };
    let steps = instance.fake.react(Call::CreateSession {
        promise,
        session_type,
        init_data_type,
        init_data,
    });
    instance.run(steps);
}

/// The fake's [`CdmCalls::update_session`]; its safety contract is [`CdmCalls`]'s.
unsafe extern "C" fn fake_update_session(
    cdm: *mut c_void,
    promise: u32,
    session: *const c_char,
    session_size: u32,
    response: *const u8,
    response_size: u32,
) {
    // SAFETY: the table's contract: a live instance, on the CDM thread, and both byte strings.
    let (instance, session, response) = unsafe {
        (
            instance(cdm),
            read(session.cast(), session_size),
            read(response, response_size),
        )
    };
    let steps = instance.fake.react(Call::UpdateSession {
        promise,
        session,
        response,
    });
    instance.run(steps);
}

/// The fake's [`CdmCalls::close_session`]; its safety contract is [`CdmCalls`]'s.
unsafe extern "C" fn fake_close_session(
    cdm: *mut c_void,
    promise: u32,
    session: *const c_char,
    session_size: u32,
) {
    // SAFETY: the table's contract: a live instance, on the CDM thread, and the session id.
    let (instance, session) = unsafe { (instance(cdm), read(session.cast(), session_size)) };
    let steps = instance.fake.react(Call::CloseSession { promise, session });
    instance.run(steps);
}

/// The fake's [`CdmCalls::timer_expired`]; its safety contract is [`CdmCalls`]'s.
unsafe extern "C" fn fake_timer_expired(cdm: *mut c_void, context: *mut c_void) {
    // SAFETY: the table's contract: a live instance, on the CDM thread.
    let instance = unsafe { instance(cdm) };
    if let Some(steps) = instance.later.remove(&(context as usize)) {
        instance.run(steps);
    }
}

/// The fake's [`CdmCalls::decrypt`]; its safety contract is [`CdmCalls`]'s.
unsafe extern "C" fn fake_decrypt(
    cdm: *mut c_void,
    input: *const DecryptInput,
    output: *mut DecryptOutput,
) -> u32 {
    // SAFETY: the table's contract: a live instance, on the CDM thread, and an input whose
    // pointers are valid for the call.
    let (instance, input) = unsafe { (instance(cdm), &*input) };
    // SAFETY: the sample is `data_size` bytes, the subsample runs `subsample_count` entries.
    let data = unsafe { std::slice::from_raw_parts(input.data, input.data_size as usize) };
    let subsamples: Vec<(u32, u32)> = if input.subsample_count == 0 {
        Vec::new()
    } else {
        // SAFETY: as above.
        unsafe { std::slice::from_raw_parts(input.subsamples, input.subsample_count as usize) }
            .iter()
            .map(|run| (run.clear_bytes, run.cipher_bytes))
            .collect()
    };
    let call = DecryptCall {
        size: data.len(),
        scheme: input.scheme,
        // SAFETY: the key id and the IV are their sizes' bytes.
        key_id: unsafe { read(input.key_id, input.key_id_size) },
        // SAFETY: as above.
        iv: unsafe { read(input.iv, input.iv_size) },
        subsamples: subsamples.clone(),
        crypt: input.crypt_byte_block,
        skip: input.skip_byte_block,
        timestamp: input.timestamp,
    };
    let steps = instance.fake.react(Call::Decrypt(call));
    let plan = instance.run(steps);
    if plan.status != 0 {
        return plan.status;
    }
    let mut clear = unmask(data, &subsamples);
    clear.resize(plan.length.unwrap_or(data.len()), 0);
    instance.fake.buffers_held.fetch_add(1, Ordering::Relaxed);
    let buffer = Box::new(FakeBuffer {
        bytes: clear,
        fake: Arc::clone(&instance.fake),
    });
    // SAFETY: the host passes an output to fill.
    let output = unsafe { &mut *output };
    output.data = buffer.bytes.as_ptr();
    output.size = buffer.bytes.len() as u32;
    output.buffer = Box::into_raw(buffer).cast();
    0
}

/// A decrypt's cleartext, handed to the host until it releases it.
struct FakeBuffer {
    bytes: Vec<u8>,
    fake: Arc<Fake>,
}

/// The fake's [`CdmCalls::release_buffer`]; its safety contract is [`CdmCalls`]'s.
unsafe extern "C" fn fake_release_buffer(buffer: *mut c_void) {
    // SAFETY: the host releases each buffer `fake_decrypt` handed it, once.
    let buffer = unsafe { Box::from_raw(buffer.cast::<FakeBuffer>()) };
    buffer.fake.buffers_held.fetch_sub(1, Ordering::Relaxed);
}

/// The fake's [`CdmCalls::storage_id`]; its safety contract is [`CdmCalls`]'s.
unsafe extern "C" fn fake_storage_id(
    cdm: *mut c_void,
    version: u32,
    storage_id: *const u8,
    size: u32,
) {
    // SAFETY: the table's contract: a live instance, on the CDM thread.
    let instance = unsafe { instance(cdm) };
    let given = !storage_id.is_null() || size != 0;
    let steps = instance.fake.react(Call::StorageId { version, given });
    instance.run(steps);
}

/// The fake's [`CdmCalls::destroy`]; its safety contract is [`CdmCalls`]'s.
unsafe extern "C" fn fake_destroy(cdm: *mut c_void) {
    // SAFETY: the table's contract: an instance `fake_create` returned, destroyed once.
    let instance = unsafe { Box::from_raw(cdm.cast::<FakeInstance>()) };
    instance.fake.record(Call::Destroy);
}

/// Applies the fake's mask to the protected bytes: both how a test encrypts a sample and how
/// the fake decrypts it.
pub(crate) fn unmask(data: &[u8], subsamples: &[(u32, u32)]) -> Vec<u8> {
    let mut out = data.to_vec();
    if subsamples.is_empty() {
        out.iter_mut().for_each(|byte| *byte ^= MASK);
        return out;
    }
    let mut at = 0;
    for &(clear, protected) in subsamples {
        at += clear as usize;
        let end = (at + protected as usize).min(out.len());
        out[at.min(end)..end]
            .iter_mut()
            .for_each(|byte| *byte ^= MASK);
        at = end;
    }
    out
}

/// A CDM that does everything right: it initialises, opens each session as `session-<n>` with
/// the challenge `challenge`, makes usable the key id the licence holds, and closes.
pub(crate) fn well_behaved() -> impl FnMut(&Call) -> Vec<Step> + Send + 'static {
    let mut opened = 0;
    move |call| match call {
        Call::Initialize { .. } => vec![Step::Initialized(true)],
        Call::CreateSession { promise, .. } => {
            opened += 1;
            let session = format!("session-{opened}").into_bytes();
            vec![
                Step::NewSession {
                    promise: *promise,
                    session: session.clone(),
                },
                Step::Message {
                    session,
                    kind: 0,
                    message: b"challenge".to_vec(),
                },
            ]
        }
        Call::UpdateSession {
            promise,
            session,
            response,
        } => vec![
            Step::Keys {
                session: session.clone(),
                keys: vec![(response.clone(), 0)],
            },
            Step::Resolve(*promise),
        ],
        Call::CloseSession { promise, session } => {
            vec![
                Step::Resolve(*promise),
                Step::Closed {
                    session: session.clone(),
                },
            ]
        }
        _ => Vec::new(),
    }
}
