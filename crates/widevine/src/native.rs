//! The browser's CDM, loaded and driven through the C++ shim.

use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard};

use crate::{Error, Result};

unsafe extern "C" {
    fn wv_open(so_path: *const c_char) -> c_int;
    fn wv_challenge(
        init_data: *const u8,
        len: u32,
        out: *mut *mut u8,
        out_len: *mut u32,
        out_type: *mut u32,
        out_sid: *mut *mut u8,
        out_sid_len: *mut u32,
    ) -> c_int;
    fn wv_close(sid: *const u8, sid_len: u32) -> c_int;
    fn wv_update(sid: *const u8, sid_len: u32, license: *const u8, len: u32) -> c_int;
    #[allow(clippy::too_many_arguments)]
    fn wv_decrypt(
        data: *const u8,
        data_size: u32,
        key_id: *const u8,
        key_id_size: u32,
        iv: *const u8,
        iv_size: u32,
        subs: *const u32,
        num_subs: u32,
        out: *mut *mut u8,
        out_len: *mut u32,
    ) -> c_int;
    #[allow(clippy::too_many_arguments)]
    fn wv_decrypt_scheme(
        data: *const u8,
        data_size: u32,
        key_id: *const u8,
        key_id_size: u32,
        iv: *const u8,
        iv_size: u32,
        subs: *const u32,
        num_subs: u32,
        cbcs: c_int,
        crypt_blocks: u32,
        skip_blocks: u32,
        out: *mut *mut u8,
        out_len: *mut u32,
    ) -> c_int;
    fn wv_free(p: *mut u8);
}

/// The shim drives one process-wide CDM instance, which is not re-entrant. Each
/// hold is a single call (a decrypt is about a millisecond), never a whole track.
static CALL_LOCK: Mutex<()> = Mutex::new(());

fn call_lock() -> MutexGuard<'static, ()> {
    CALL_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Calls other than decrypts waiting for the lock. A track decrypting would
/// otherwise take it back the moment it lets go, sample after sample, and the
/// licence of the track the user just picked would wait behind it.
static URGENT: AtomicUsize = AtomicUsize::new(0);

/// The lock for a session's opening, licence or closing: ahead of decrypts.
fn urgent_lock() -> MutexGuard<'static, ()> {
    URGENT.fetch_add(1, Ordering::AcqRel);
    let guard = call_lock();
    URGENT.fetch_sub(1, Ordering::AcqRel);
    guard
}

/// The lock for one decrypt, once no other call is waiting for it.
fn decrypt_lock() -> MutexGuard<'static, ()> {
    // A licence takes a few milliseconds of CDM time; give up waiting long
    // after, in case one hangs.
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(200);
    while URGENT.load(Ordering::Acquire) > 0 && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_micros(200));
    }
    call_lock()
}

/// Call before the process exits. The CDM's own destructors run at exit, and a
/// thread still closing a session meanwhile crashes inside it; this takes the
/// call lock for good, so such threads wait on it instead.
pub fn shutdown() {
    std::mem::forget(call_lock());
}

/// Take ownership of a buffer the shim allocated.
unsafe fn take(out: *mut u8, len: u32) -> Vec<u8> {
    if out.is_null() {
        return Vec::new();
    }
    let v = unsafe { std::slice::from_raw_parts(out, len as usize) }.to_vec();
    unsafe { wv_free(out) };
    v
}

/// The loaded CDM. Opening it again is a no-op, so this is a cheap handle.
#[derive(Debug, Clone, Copy)]
pub struct Cdm(());

impl Cdm {
    pub fn open(path: &Path) -> Result<Self> {
        let c = CString::new(path.as_os_str().as_encoded_bytes())
            .map_err(|_| Error::Load(path.to_owned()))?;
        let _guard = urgent_lock();
        match unsafe { wv_open(c.as_ptr()) } {
            0 => Ok(Self(())),
            1 => Err(Error::Load(path.to_owned())),
            2 => Err(Error::NotACdm(path.to_owned())),
            3 => Err(Error::Interface),
            _ => Err(Error::Init),
        }
    }

    /// Open a session for `pssh` and return it with the licence challenge to send
    /// to the licence server.
    pub fn start_session(&self, pssh: &[u8]) -> Result<(Session, Vec<u8>)> {
        const LICENSE_REQUEST: u32 = 0;
        const INDIVIDUALIZATION_REQUEST: u32 = 3;

        let mut out = std::ptr::null_mut();
        let mut len = 0u32;
        let mut msg_type = 0u32;
        let mut sid = std::ptr::null_mut();
        let mut sid_len = 0u32;
        let _guard = urgent_lock();
        let rc = unsafe {
            wv_challenge(
                pssh.as_ptr(),
                pssh.len() as u32,
                &mut out,
                &mut len,
                &mut msg_type,
                &mut sid,
                &mut sid_len,
            )
        };
        match rc {
            0 => {}
            11 => return Err(Error::BadInitData),
            n => return Err(Error::Call("challenge", n)),
        }
        let challenge = unsafe { take(out, len) };
        let session = Session {
            id: unsafe { take(sid, sid_len) },
        };
        match msg_type {
            LICENSE_REQUEST => Ok((session, challenge)),
            INDIVIDUALIZATION_REQUEST => Err(Error::Unprovisioned),
            n => Err(Error::Call("challenge message type", n as c_int)),
        }
    }
}

/// A CDM session and the content keys loaded into it. The keys live as long as
/// the session: dropping it closes the session and releases them.
#[derive(Debug)]
pub struct Session {
    id: Vec<u8>,
}

impl Session {
    /// Load the licence server's response, giving the CDM the content keys.
    pub fn update(&self, license: &[u8]) -> Result<()> {
        let _guard = urgent_lock();
        match unsafe {
            wv_update(
                self.id.as_ptr(),
                self.id.len() as u32,
                license.as_ptr(),
                license.len() as u32,
            )
        } {
            0 => Ok(()),
            21 => Err(Error::LicenseRejected),
            22 => Err(Error::NoKey),
            n => Err(Error::Call("update", n)),
        }
    }

    /// Decrypt one CENC (AES-CTR) sample. `subsamples` are `(clear, encrypted)`
    /// byte runs; empty means the whole sample is encrypted.
    pub fn decrypt(
        &self,
        data: &[u8],
        key_id: &[u8],
        iv: &[u8],
        subsamples: &[(u32, u32)],
    ) -> Result<Vec<u8>> {
        let subs: Vec<u32> = subsamples.iter().flat_map(|&(c, e)| [c, e]).collect();
        let mut out = std::ptr::null_mut();
        let mut len = 0u32;
        let _guard = decrypt_lock();
        let rc = unsafe {
            wv_decrypt(
                data.as_ptr(),
                data.len() as u32,
                key_id.as_ptr(),
                key_id.len() as u32,
                iv.as_ptr(),
                iv.len() as u32,
                subs.as_ptr(),
                subsamples.len() as u32,
                &mut out,
                &mut len,
            )
        };
        match rc {
            0 => Ok(unsafe { take(out, len) }),
            32 => Err(Error::NoKey),
            n => Err(Error::Call("decrypt", n)),
        }
    }
}

impl Session {
    /// Decrypt one CBCS (AES-CBC) sample, encrypting `crypt` blocks of every
    /// `crypt + skip` (0 and 0: every whole block, as audio does).
    /// `subsamples` are `(clear, encrypted)` byte runs, as for CENC.
    pub fn decrypt_cbcs(
        &self,
        data: &[u8],
        key_id: &[u8],
        iv: &[u8],
        pattern: (u32, u32),
        subsamples: &[(u32, u32)],
    ) -> Result<Vec<u8>> {
        let subs: Vec<u32> = subsamples.iter().flat_map(|&(c, e)| [c, e]).collect();
        let mut out = std::ptr::null_mut();
        let mut len = 0u32;
        let _guard = decrypt_lock();
        let rc = unsafe {
            wv_decrypt_scheme(
                data.as_ptr(),
                data.len() as u32,
                key_id.as_ptr(),
                key_id.len() as u32,
                iv.as_ptr(),
                iv.len() as u32,
                subs.as_ptr(),
                subsamples.len() as u32,
                1,
                pattern.0,
                pattern.1,
                &mut out,
                &mut len,
            )
        };
        match rc {
            0 => Ok(unsafe { take(out, len) }),
            32 => Err(Error::NoKey),
            n => Err(Error::Call("decrypt", n)),
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _guard = urgent_lock();
        unsafe { wv_close(self.id.as_ptr(), self.id.len() as u32) };
    }
}
