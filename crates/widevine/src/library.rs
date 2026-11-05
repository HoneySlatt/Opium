//! Opening the CDM's library (§2), and what the CDM thread needs of a loaded CDM.

use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use crate::Error;
use crate::boundary::CdmCalls;

/// What the CDM thread needs of a loaded CDM, whoever provides it: the library's module
/// functions, the table of calls for the interface version the host speaks, and what that
/// table's `create` is given.
pub(crate) struct Library {
    /// The calls on an instance; the C++ layer's, or a fake's in tests.
    pub calls: &'static CdmCalls,
    /// Passed to `calls.create`: the library's `CreateCdmInstance`, or a fake's own state.
    pub entry: Entry,
    /// The library's module functions.
    pub module: Box<dyn Module>,
}

/// The pointer `CdmCalls::create` is given.
pub(crate) struct Entry(pub *mut c_void);

// SAFETY: the pointer is a function's address or a fake's state that outlives the library; it is
// only passed to `create`, on the CDM thread.
unsafe impl Send for Entry {}

/// The library-wide functions of a CDM, called on the CDM thread.
pub(crate) trait Module: Send {
    /// `InitializeCdmModule_4`: once, before the first instance.
    fn initialize(&mut self);
    /// `DeinitializeCdmModule`, when the library exports it: once, at shutdown.
    fn deinitialize(&mut self);
    /// `GetCdmVersion`, for diagnostics.
    fn version(&mut self) -> String;
}

/// The functions every CDM exports, in the order they are looked for.
const INITIALIZE_MODULE: &CStr = c"InitializeCdmModule_4";
const CREATE_INSTANCE: &CStr = c"CreateCdmInstance";
const GET_VERSION: &CStr = c"GetCdmVersion";
const DEINITIALIZE_MODULE: &CStr = c"DeinitializeCdmModule";

/// `dlopen`'s flags on Linux: resolve every symbol now, and keep them local to the handle so
/// that they never replace the program's.
const RTLD_NOW: c_int = 2;
const RTLD_LOCAL: c_int = 0;

unsafe extern "C" {
    /// The C++ layer's table for interface 11 (shim/cdm_11.cc).
    fn widevine_cdm_calls_11() -> *const CdmCalls;
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlerror() -> *mut c_char;
}

/// The C++ layer's calls for the interface version this host speaks, 11: the one place that
/// names it. Another version is another C++ file and another table, chosen here.
pub(crate) fn cpp_calls() -> &'static CdmCalls {
    // SAFETY: the C++ layer returns the address of a static table laid out as `CdmCalls`.
    unsafe { &*widevine_cdm_calls_11() }
}

/// A CDM's shared library, opened and never closed: no thread can be inside it when the
/// process's exit handlers run.
struct SharedLibrary {
    initialize_module: unsafe extern "C" fn(),
    deinitialize_module: Option<unsafe extern "C" fn()>,
    get_version: unsafe extern "C" fn() -> *const c_char,
}

impl Module for SharedLibrary {
    fn initialize(&mut self) {
        // SAFETY: `InitializeCdmModule_4` takes nothing and returns nothing; the CDM thread
        // calls it once, before the first `CreateCdmInstance`.
        unsafe { (self.initialize_module)() }
    }

    fn deinitialize(&mut self) {
        if let Some(deinitialize) = self.deinitialize_module {
            // SAFETY: `DeinitializeCdmModule` takes nothing and returns nothing; the CDM thread
            // calls it once, after the instance is destroyed.
            unsafe { deinitialize() }
        }
    }

    fn version(&mut self) -> String {
        // SAFETY: `GetCdmVersion` takes nothing and returns a C string the library owns, or
        // null.
        let version = unsafe { (self.get_version)() };
        if version.is_null() {
            return String::new();
        }
        // SAFETY: not null, it is a C string the library keeps for as long as it is open, and
        // it is never closed.
        unsafe { CStr::from_ptr(version) }
            .to_string_lossy()
            .into_owned()
    }
}

/// Opens the library at `path` with `calls`, the C++ layer's table for the interface version
/// the host speaks.
///
/// # Errors
///
/// [`Error::LibraryNotOpened`] with the loader's message; [`Error::NotACdm`] naming the first
/// missing function.
pub(crate) fn open(path: &Path, calls: &'static CdmCalls) -> Result<Library, Error> {
    let name = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| Error::LibraryNotOpened("the path holds a NUL byte".to_owned()))?;
    // SAFETY: `name` is a C string. Opening runs the library's initialisers: loading a CDM
    // trusts it, as the specification's §2 says.
    let handle = unsafe { dlopen(name.as_ptr(), RTLD_NOW | RTLD_LOCAL) };
    if handle.is_null() {
        return Err(Error::LibraryNotOpened(loader_message()));
    }
    library_from(handle, calls)
}

/// The CDM in an opened library, or which of its functions is missing.
pub(crate) fn library_from(
    handle: *mut c_void,
    calls: &'static CdmCalls,
) -> Result<Library, Error> {
    let find = |name: &CStr| {
        // SAFETY: `handle` is a handle `dlopen` returned, never closed; `name` is a C string.
        let symbol = unsafe { dlsym(handle, name.as_ptr()) };
        (!symbol.is_null()).then_some(symbol)
    };
    let missing = |name: &'static CStr| Error::NotACdm(name.to_str().unwrap_or("a CDM function"));
    let initialize_module = find(INITIALIZE_MODULE).ok_or_else(|| missing(INITIALIZE_MODULE))?;
    let create_instance = find(CREATE_INSTANCE).ok_or_else(|| missing(CREATE_INSTANCE))?;
    let get_version = find(GET_VERSION).ok_or_else(|| missing(GET_VERSION))?;
    let deinitialize_module = find(DEINITIALIZE_MODULE);
    let module = SharedLibrary {
        // SAFETY: Chromium's header declares `InitializeCdmModule_4` as `void ()`; a symbol's
        // address is the function's.
        initialize_module: unsafe {
            std::mem::transmute::<*mut c_void, unsafe extern "C" fn()>(initialize_module)
        },
        deinitialize_module: deinitialize_module.map(|symbol| {
            // SAFETY: the header declares `DeinitializeCdmModule` as `void ()`.
            unsafe { std::mem::transmute::<*mut c_void, unsafe extern "C" fn()>(symbol) }
        }),
        // SAFETY: the header declares `GetCdmVersion` as `const char* ()`.
        get_version: unsafe {
            std::mem::transmute::<*mut c_void, unsafe extern "C" fn() -> *const c_char>(get_version)
        },
    };
    Ok(Library {
        calls,
        entry: Entry(create_instance),
        module: Box::new(module),
    })
}

/// `dlerror`'s message, for the last failure on this thread.
fn loader_message() -> String {
    // SAFETY: `dlerror` returns this thread's last error as a C string, or null.
    let message = unsafe { dlerror() };
    if message.is_null() {
        return "the loader gave no reason".to_owned();
    }
    // SAFETY: not null, it is a C string valid until the next `dl` call on this thread.
    unsafe { CStr::from_ptr(message) }
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calls() -> &'static CdmCalls {
        cpp_calls()
    }

    #[test]
    fn a_program_without_the_cdm_functions_is_not_a_cdm() {
        // SAFETY: a null name opens the program itself, which is already loaded.
        let program = unsafe { dlopen(std::ptr::null(), RTLD_NOW | RTLD_LOCAL) };
        assert!(!program.is_null());
        match library_from(program, calls()) {
            Err(error) => {
                assert_eq!(error, Error::NotACdm("InitializeCdmModule_4"));
                assert!(error.to_string().contains("InitializeCdmModule_4"));
            }
            Ok(_) => panic!("the test program is not a CDM"),
        }
    }

    #[test]
    fn a_file_that_is_not_a_library_is_refused_with_the_loaders_message() {
        let folder = crate::tests::TestFolder::new("not-a-library");
        let path = folder.path().join("libwidevinecdm.so");
        std::fs::write(&path, b"not a library").unwrap();
        match open(&path, calls()) {
            Err(Error::LibraryNotOpened(message)) => assert!(!message.is_empty()),
            Err(other) => panic!("unexpected error {other}"),
            Ok(_) => panic!("a text file opened as a library"),
        }
        match open(folder.path(), calls()) {
            Err(Error::LibraryNotOpened(_)) => {}
            Err(other) => panic!("unexpected error {other}"),
            Ok(_) => panic!("a folder opened as a library"),
        }
    }
}
