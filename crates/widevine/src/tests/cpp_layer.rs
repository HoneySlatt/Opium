//! The C++ layer, compiled and linked into the tests although no CDM is loaded: its table, and
//! its `create` driving a stand-in for the library's `CreateCdmInstance`.

use std::ffi::{c_char, c_int, c_void};
use std::sync::Mutex;

use crate::boundary::{CdmCalls, HOST_CALLS};
use crate::library::cpp_calls;

/// `GetCdmHostFunc` of the header.
type GetCdmHost =
    unsafe extern "C" fn(host_interface_version: c_int, user_data: *mut c_void) -> *mut c_void;

/// What the stand-in saw.
#[derive(Debug, Default, PartialEq)]
struct Seen {
    interface_version: c_int,
    key_system: Vec<u8>,
    /// What the host lookup gave for versions 10, 11 and 12.
    hosts: [bool; 3],
    host_is_user_data: bool,
}

static SEEN: Mutex<Option<Seen>> = Mutex::new(None);

/// Stands for `CreateCdmInstance`: records what it is given, asks the host lookup for three
/// versions, and creates nothing.
unsafe extern "C" fn create_nothing(
    interface_version: c_int,
    key_system: *const c_char,
    key_system_size: u32,
    get_host: GetCdmHost,
    user_data: *mut c_void,
) -> *mut c_void {
    // SAFETY: the C++ layer passes the key system's bytes.
    let key_system =
        unsafe { std::slice::from_raw_parts(key_system.cast::<u8>(), key_system_size as usize) }
            .to_vec();
    // SAFETY: the C++ layer's lookup takes any version and the user data it gave.
    let hosts = [10, 11, 12].map(|version| unsafe { get_host(version, user_data) });
    let seen = Seen {
        interface_version,
        key_system,
        hosts: hosts.map(|host| !host.is_null()),
        host_is_user_data: hosts[1] == user_data,
    };
    *SEEN.lock().unwrap() = Some(seen);
    std::ptr::null_mut()
}

#[test]
fn the_cpp_table_matches_the_rust_boundary() {
    let calls = cpp_calls();
    assert_eq!(calls.size as usize, size_of::<CdmCalls>());
    assert_eq!(calls.interface_version, 11);
}

#[test]
fn the_cpp_layer_asks_for_interface_11_and_its_host_lookup_answers_11_only() {
    let calls = cpp_calls();
    let key_system = b"com.widevine.alpha";
    // SAFETY: the entry is a function with `CreateCdmInstance`'s signature; the key system is
    // its length's bytes; the host pointer is never called back, as no instance is made.
    let instance = unsafe {
        (calls.create)(
            create_nothing as *mut c_void,
            key_system.as_ptr().cast(),
            key_system.len() as u32,
            std::ptr::dangling_mut(),
            &HOST_CALLS,
        )
    };
    assert!(
        instance.is_null(),
        "no instance when the library gives none"
    );
    assert_eq!(
        SEEN.lock().unwrap().take(),
        Some(Seen {
            interface_version: 11,
            key_system: key_system.to_vec(),
            hosts: [false, true, false],
            host_is_user_data: true,
        })
    );
}
