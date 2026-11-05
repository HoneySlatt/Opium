//! The C boundary between the host and a CDM: the structures of `shim/boundary.h`, field for
//! field, and the host's callbacks.
//!
//! Behind [`CdmCalls`] stands either the C++ layer of one interface version and the CDM it
//! wraps, or, in tests, a fake CDM written in Rust: the host cannot tell them apart. Nothing
//! here knows an interface version.
//!
//! The CDM calls the host back through [`HOST_CALLS`]. Each callback copies what it is given
//! into an [`Event`] in the CDM thread's [`Inbox`], and the CDM thread acts on the events once
//! the call into the CDM has returned: no callback re-enters the host's state, none calls into
//! the CDM, and none lets a panic cross the boundary.

use std::cell::{Cell, RefCell};
use std::ffi::{c_char, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::time::{SystemTime, UNIX_EPOCH};

/// `cdm::SessionType::kTemporary`.
pub(crate) const SESSION_TEMPORARY: u32 = 0;
/// `cdm::InitDataType::kCenc`.
pub(crate) const INIT_DATA_CENC: u32 = 0;
/// `cdm::EncryptionScheme::kCenc`.
pub(crate) const SCHEME_CENC: u32 = 1;
/// `cdm::EncryptionScheme::kCbcs`.
pub(crate) const SCHEME_CBCS: u32 = 2;

/// `cdm::MessageType` values.
pub(crate) const MESSAGE_LICENCE_REQUEST: u32 = 0;
pub(crate) const MESSAGE_LICENCE_RENEWAL: u32 = 1;
pub(crate) const MESSAGE_LICENCE_RELEASE: u32 = 2;
pub(crate) const MESSAGE_INDIVIDUALIZATION_REQUEST: u32 = 3;

/// The `WV_UNANSWERED_` values of `boundary.h`: calls the host does not answer.
pub(crate) const UNANSWERED_QUERY_OUTPUT_PROTECTION: u32 = 0;
pub(crate) const UNANSWERED_ENABLE_OUTPUT_PROTECTION: u32 = 1;
pub(crate) const UNANSWERED_PLATFORM_CHALLENGE: u32 = 2;
pub(crate) const UNANSWERED_FILE_IO: u32 = 3;
#[cfg(test)]
pub(crate) const UNANSWERED_METRICS: u32 = 5;

/// `WvKeyInfo`: one key's status, laid out as `cdm::KeyInformation`.
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct KeyInfo {
    pub key_id: *const u8,
    pub key_id_size: u32,
    pub status: u32,
    pub system_code: u32,
}

/// `WvSubsample`: one subsample run, laid out as `cdm::SubsampleEntry`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Subsample {
    pub clear_bytes: u32,
    pub cipher_bytes: u32,
}

/// `WvDecryptInput`: one sample to decrypt.
#[repr(C)]
pub(crate) struct DecryptInput {
    pub data: *const u8,
    pub data_size: u32,
    pub scheme: u32,
    pub key_id: *const u8,
    pub key_id_size: u32,
    pub iv: *const u8,
    pub iv_size: u32,
    pub subsamples: *const Subsample,
    pub subsample_count: u32,
    pub crypt_byte_block: u32,
    pub skip_byte_block: u32,
    pub timestamp: i64,
}

/// `WvDecryptOutput`: the CDM's buffer after a decrypt, if it set one.
#[repr(C)]
pub(crate) struct DecryptOutput {
    pub data: *const u8,
    pub size: u32,
    pub buffer: *mut c_void,
}

/// `WvCdmCalls`: every call the host makes on a CDM instance (`cdm`), and how to make one.
///
/// # Safety, for every entry
///
/// `cdm` is an instance `create` returned and `destroy` has not ended; the calls are made on
/// the CDM thread only; pointers with sizes point to that many readable bytes for the call's
/// duration.
#[repr(C)]
pub(crate) struct CdmCalls {
    /// `size_of::<CdmCalls>()` as the other side sees it.
    pub size: u32,
    /// The interface version behind this table, for errors only.
    pub interface_version: u32,
    pub create: unsafe extern "C" fn(
        entry: *mut c_void,
        key_system: *const c_char,
        key_system_size: u32,
        host: *mut c_void,
        host_calls: *const HostCalls,
    ) -> *mut c_void,
    pub initialize: unsafe extern "C" fn(
        cdm: *mut c_void,
        allow_distinctive_identifier: bool,
        allow_persistent_state: bool,
        use_hw_secure_codecs: bool,
    ),
    pub create_session: unsafe extern "C" fn(
        cdm: *mut c_void,
        promise_id: u32,
        session_type: u32,
        init_data_type: u32,
        init_data: *const u8,
        init_data_size: u32,
    ),
    pub update_session: unsafe extern "C" fn(
        cdm: *mut c_void,
        promise_id: u32,
        session_id: *const c_char,
        session_id_size: u32,
        response: *const u8,
        response_size: u32,
    ),
    pub close_session: unsafe extern "C" fn(
        cdm: *mut c_void,
        promise_id: u32,
        session_id: *const c_char,
        session_id_size: u32,
    ),
    pub timer_expired: unsafe extern "C" fn(cdm: *mut c_void, context: *mut c_void),
    pub decrypt: unsafe extern "C" fn(
        cdm: *mut c_void,
        input: *const DecryptInput,
        output: *mut DecryptOutput,
    ) -> u32,
    /// Destroys a buffer a successful decrypt left in its output.
    pub release_buffer: unsafe extern "C" fn(buffer: *mut c_void),
    pub storage_id: unsafe extern "C" fn(
        cdm: *mut c_void,
        version: u32,
        storage_id: *const u8,
        storage_id_size: u32,
    ),
    pub destroy: unsafe extern "C" fn(cdm: *mut c_void),
}

/// `WvHostCalls`: every call a CDM makes on its host; `host` is the [`Inbox`] given to
/// `create`.
///
/// # Safety, for every entry
///
/// Each pointer with a size points to that many readable bytes for the call's duration, or is
/// null; `keys` points to `key_count` entries laid out as `cdm::KeyInformation`. `host` may be
/// anything: it is compared with the calling thread's inbox before it is read.
#[repr(C)]
pub(crate) struct HostCalls {
    pub set_timer: unsafe extern "C" fn(host: *mut c_void, delay_ms: i64, context: *mut c_void),
    pub wall_time: unsafe extern "C" fn(host: *mut c_void) -> f64,
    pub on_initialized: unsafe extern "C" fn(host: *mut c_void, success: bool),
    pub on_resolve_key_status:
        unsafe extern "C" fn(host: *mut c_void, promise_id: u32, key_status: u32),
    pub on_resolve_new_session: unsafe extern "C" fn(
        host: *mut c_void,
        promise_id: u32,
        session_id: *const c_char,
        session_id_size: u32,
    ),
    pub on_resolve: unsafe extern "C" fn(host: *mut c_void, promise_id: u32),
    pub on_reject: unsafe extern "C" fn(
        host: *mut c_void,
        promise_id: u32,
        exception: u32,
        system_code: u32,
        message: *const c_char,
        message_size: u32,
    ),
    pub on_session_message: unsafe extern "C" fn(
        host: *mut c_void,
        session_id: *const c_char,
        session_id_size: u32,
        message_type: u32,
        message: *const c_char,
        message_size: u32,
    ),
    pub on_keys_change: unsafe extern "C" fn(
        host: *mut c_void,
        session_id: *const c_char,
        session_id_size: u32,
        has_additional_usable_key: bool,
        keys: *const KeyInfo,
        key_count: u32,
    ),
    pub on_expiration_change: unsafe extern "C" fn(
        host: *mut c_void,
        session_id: *const c_char,
        session_id_size: u32,
        new_expiry_time: f64,
    ),
    pub on_session_closed:
        unsafe extern "C" fn(host: *mut c_void, session_id: *const c_char, session_id_size: u32),
    pub request_storage_id: unsafe extern "C" fn(host: *mut c_void, version: u32),
    pub unanswered: unsafe extern "C" fn(host: *mut c_void, request: u32),
}

/// The host's callbacks, given to every CDM instance.
pub(crate) static HOST_CALLS: HostCalls = HostCalls {
    set_timer,
    wall_time,
    on_initialized,
    on_resolve_key_status,
    on_resolve_new_session,
    on_resolve,
    on_reject,
    on_session_message,
    on_keys_change,
    on_expiration_change,
    on_session_closed,
    request_storage_id,
    unanswered,
};

/// A timer's context, as the CDM gave it: handed back as it was, never read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TimerContext(pub *mut c_void);

/// What a callback told the host, copied out of the CDM's memory.
#[derive(Debug)]
pub(crate) enum Event {
    TimerSet {
        delay_ms: i64,
        context: TimerContext,
    },
    Initialized {
        success: bool,
    },
    /// The host never asks for a key status, so such an answer is always ignored.
    KeyStatusResolved,
    NewSessionResolved {
        promise: u32,
        session: Vec<u8>,
    },
    Resolved {
        promise: u32,
    },
    Rejected {
        promise: u32,
        exception: u32,
        system_code: u32,
        message: String,
    },
    Message {
        session: Vec<u8>,
        kind: u32,
        message: Vec<u8>,
    },
    KeysChanged {
        session: Vec<u8>,
        keys: Vec<(Vec<u8>, u32)>,
    },
    ExpirationChanged {
        session: Vec<u8>,
        expiry: f64,
    },
    Closed {
        session: Vec<u8>,
    },
    StorageIdRequested {
        version: u32,
    },
    Unanswered {
        request: u32,
    },
}

/// Where the callbacks of one CDM thread put their events.
///
/// Its address is the `host` pointer the CDM is given. Each thread records the inbox it owns
/// in [`CURRENT_INBOX`]; a callback whose `host` is not the calling thread's inbox, because it
/// comes from another thread or names something else, is dropped without being read.
pub(crate) struct Inbox {
    events: RefCell<Vec<Event>>,
}

thread_local! {
    /// The inbox of the CDM thread running here, or null.
    static CURRENT_INBOX: Cell<*const Inbox> = const { Cell::new(ptr::null()) };
}

impl Inbox {
    /// A new inbox, made the calling thread's own: the thread that owns the CDM.
    pub(crate) fn for_this_thread() -> Box<Inbox> {
        let inbox = Box::new(Inbox {
            events: RefCell::new(Vec::new()),
        });
        let address: *const Inbox = &*inbox;
        CURRENT_INBOX.with(|current| current.set(address));
        inbox
    }

    /// The pointer given to the CDM as its host.
    pub(crate) fn as_host(&self) -> *mut c_void {
        ptr::from_ref(self).cast_mut().cast()
    }

    /// The events recorded since the last call, in order.
    pub(crate) fn take(&self) -> Vec<Event> {
        match self.events.try_borrow_mut() {
            Ok(mut events) => std::mem::take(&mut *events),
            Err(_) => Vec::new(),
        }
    }

    /// Whether events are waiting.
    pub(crate) fn has_events(&self) -> bool {
        self.events
            .try_borrow()
            .is_ok_and(|events| !events.is_empty())
    }
}

impl Drop for Inbox {
    fn drop(&mut self) {
        let address: *const Inbox = self;
        // Ignore a thread whose locals are already gone: nothing can call back then.
        let _ = CURRENT_INBOX.try_with(|current| {
            if current.get() == address {
                current.set(ptr::null());
            }
        });
    }
}

/// Records the event `make` builds in the inbox `host` names, when it is the calling thread's.
/// A panic in `make` is caught: it never crosses the boundary.
fn record(host: *mut c_void, make: impl FnOnce() -> Event) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let owned = CURRENT_INBOX.try_with(Cell::get).unwrap_or(ptr::null());
        if owned.is_null() || owned != host.cast_const().cast::<Inbox>() {
            crate::log::line(format_args!(
                "a callback from outside the CDM thread was ignored"
            ));
            return;
        }
        let event = make();
        // SAFETY: `owned` is the inbox this thread created and still owns (`Inbox::drop`
        // clears it), so it is alive; only this thread uses it, through shared references.
        let inbox = unsafe { &*owned };
        if let Ok(mut events) = inbox.events.try_borrow_mut() {
            events.push(event);
        }
    }));
}

/// The bytes at `data`, or none when it is null.
///
/// # Safety
///
/// When not null, `data` points to `size` readable bytes for the lifetime `'a`.
unsafe fn bytes<'a>(data: *const c_char, size: u32) -> &'a [u8] {
    if data.is_null() || size == 0 {
        return &[];
    }
    // SAFETY: the caller guarantees `size` readable bytes at `data`.
    unsafe { std::slice::from_raw_parts(data.cast::<u8>(), size as usize) }
}

/// [`HostCalls::set_timer`]; its safety contract is [`HostCalls`]'s.
unsafe extern "C" fn set_timer(host: *mut c_void, delay_ms: i64, context: *mut c_void) {
    record(host, || Event::TimerSet {
        delay_ms,
        context: TimerContext(context),
    });
}

/// [`HostCalls::wall_time`]; its safety contract is [`HostCalls`]'s.
unsafe extern "C" fn wall_time(_host: *mut c_void) -> f64 {
    catch_unwind(|| match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(since) => since.as_secs_f64(),
        Err(before) => -before.duration().as_secs_f64(),
    })
    .unwrap_or(0.0)
}

/// [`HostCalls::on_initialized`]; its safety contract is [`HostCalls`]'s.
unsafe extern "C" fn on_initialized(host: *mut c_void, success: bool) {
    record(host, || Event::Initialized { success });
}

/// [`HostCalls::on_resolve_key_status`]; its safety contract is [`HostCalls`]'s.
unsafe extern "C" fn on_resolve_key_status(host: *mut c_void, _promise_id: u32, _status: u32) {
    record(host, || Event::KeyStatusResolved);
}

/// [`HostCalls::on_resolve_new_session`]; its safety contract is [`HostCalls`]'s.
unsafe extern "C" fn on_resolve_new_session(
    host: *mut c_void,
    promise_id: u32,
    session_id: *const c_char,
    session_id_size: u32,
) {
    record(host, || Event::NewSessionResolved {
        promise: promise_id,
        // SAFETY: the CDM passes the session id's bytes for the call's duration.
        session: unsafe { bytes(session_id, session_id_size) }.to_vec(),
    });
}

/// [`HostCalls::on_resolve`]; its safety contract is [`HostCalls`]'s.
unsafe extern "C" fn on_resolve(host: *mut c_void, promise_id: u32) {
    record(host, || Event::Resolved {
        promise: promise_id,
    });
}

/// [`HostCalls::on_reject`]; its safety contract is [`HostCalls`]'s.
unsafe extern "C" fn on_reject(
    host: *mut c_void,
    promise_id: u32,
    exception: u32,
    system_code: u32,
    message: *const c_char,
    message_size: u32,
) {
    record(host, || Event::Rejected {
        promise: promise_id,
        exception,
        system_code,
        // SAFETY: the CDM passes the message's bytes for the call's duration.
        message: String::from_utf8_lossy(unsafe { bytes(message, message_size) }).into_owned(),
    });
}

/// [`HostCalls::on_session_message`]; its safety contract is [`HostCalls`]'s.
unsafe extern "C" fn on_session_message(
    host: *mut c_void,
    session_id: *const c_char,
    session_id_size: u32,
    message_type: u32,
    message: *const c_char,
    message_size: u32,
) {
    record(host, || Event::Message {
        // SAFETY: the CDM passes both byte strings for the call's duration.
        session: unsafe { bytes(session_id, session_id_size) }.to_vec(),
        kind: message_type,
        // SAFETY: as above.
        message: unsafe { bytes(message, message_size) }.to_vec(),
    });
}

/// [`HostCalls::on_keys_change`]; its safety contract is [`HostCalls`]'s.
unsafe extern "C" fn on_keys_change(
    host: *mut c_void,
    session_id: *const c_char,
    session_id_size: u32,
    _has_additional_usable_key: bool,
    keys: *const KeyInfo,
    key_count: u32,
) {
    record(host, || {
        let entries: &[KeyInfo] = if keys.is_null() || key_count == 0 {
            &[]
        } else {
            // SAFETY: the CDM passes `key_count` entries at `keys` for the call's duration;
            // `KeyInfo` has `cdm::KeyInformation`'s layout, which the C++ layer checks.
            unsafe { std::slice::from_raw_parts(keys, key_count as usize) }
        };
        let keys = entries
            .iter()
            .map(|entry| {
                // SAFETY: each entry's key id is `key_id_size` bytes for the call's duration.
                let key_id = unsafe { bytes(entry.key_id.cast(), entry.key_id_size) };
                (key_id.to_vec(), entry.status)
            })
            .collect();
        Event::KeysChanged {
            // SAFETY: the CDM passes the session id's bytes for the call's duration.
            session: unsafe { bytes(session_id, session_id_size) }.to_vec(),
            keys,
        }
    });
}

/// [`HostCalls::on_expiration_change`]; its safety contract is [`HostCalls`]'s.
unsafe extern "C" fn on_expiration_change(
    host: *mut c_void,
    session_id: *const c_char,
    session_id_size: u32,
    new_expiry_time: f64,
) {
    record(host, || Event::ExpirationChanged {
        // SAFETY: the CDM passes the session id's bytes for the call's duration.
        session: unsafe { bytes(session_id, session_id_size) }.to_vec(),
        expiry: new_expiry_time,
    });
}

/// [`HostCalls::on_session_closed`]; its safety contract is [`HostCalls`]'s.
unsafe extern "C" fn on_session_closed(
    host: *mut c_void,
    session_id: *const c_char,
    session_id_size: u32,
) {
    record(host, || Event::Closed {
        // SAFETY: the CDM passes the session id's bytes for the call's duration.
        session: unsafe { bytes(session_id, session_id_size) }.to_vec(),
    });
}

/// [`HostCalls::request_storage_id`]; its safety contract is [`HostCalls`]'s.
unsafe extern "C" fn request_storage_id(host: *mut c_void, version: u32) {
    record(host, || Event::StorageIdRequested { version });
}

/// [`HostCalls::unanswered`]; its safety contract is [`HostCalls`]'s.
unsafe extern "C" fn unanswered(host: *mut c_void, request: u32) {
    record(host, || Event::Unanswered { request });
}
