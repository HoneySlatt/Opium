//! Loading (§2): idempotent, another path refused, a failed initialisation retried from the
//! library already open, loads from several threads at once, a shutdown during a load, nothing
//! after shutdown.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::channel;
use std::thread;

use super::fake::{Call, Fake, Gate, Step, well_behaved};
use super::{QUICK, TestFolder};
use crate::Error;
use crate::desktop::Loader;
use crate::library::Library;

/// Opens the fake whatever the path, counting how often.
fn opener(
    fake: &Arc<Fake>,
    opened: &Arc<AtomicUsize>,
) -> impl FnOnce(&Path) -> Result<Library, Error> {
    let fake = Arc::clone(fake);
    let opened = Arc::clone(opened);
    move |_| {
        opened.fetch_add(1, Ordering::Relaxed);
        Ok(fake.library())
    }
}

fn library_files(folder: &TestFolder) -> (std::path::PathBuf, std::path::PathBuf) {
    let first = folder.path().join("first.so");
    let second = folder.path().join("second.so");
    std::fs::write(&first, b"").unwrap();
    std::fs::write(&second, b"").unwrap();
    (first, second)
}

#[test]
fn loading_again_gives_the_same_instance_and_another_path_is_refused() {
    let folder = TestFolder::new("loader-same");
    let (first, second) = library_files(&folder);
    let fake = Fake::new(well_behaved());
    let opened = Arc::new(AtomicUsize::new(0));
    let loader = Loader::new();
    loader.load(&first, opener(&fake, &opened), QUICK).unwrap();
    let spelled_otherwise = folder.path().join(".").join("first.so");
    loader
        .load(&spelled_otherwise, opener(&fake, &opened), QUICK)
        .unwrap();
    let refused = loader
        .load(&second, opener(&fake, &opened), QUICK)
        .map(|_| ());
    assert_eq!(
        refused,
        Err(Error::AnotherCdmLoaded(
            std::fs::canonicalize(&first).unwrap()
        ))
    );
    assert_eq!(opened.load(Ordering::Relaxed), 1);
    loader.shutdown();
    let creates = fake
        .calls()
        .iter()
        .filter(|call| matches!(call, Call::Create { .. }))
        .count();
    assert_eq!(creates, 1);
}

#[test]
fn a_failed_initialisation_is_tried_again_from_the_library_already_open() {
    let folder = TestFolder::new("loader-retry");
    let (first, _) = library_files(&folder);
    let mut attempts = 0;
    let mut base = well_behaved();
    let fake = Fake::new(move |call: &Call| match call {
        Call::Initialize { .. } => {
            attempts += 1;
            vec![Step::Initialized(attempts > 1)]
        }
        other => base(other),
    });
    let opened = Arc::new(AtomicUsize::new(0));
    let loader = Loader::new();
    let failed = loader
        .load(&first, opener(&fake, &opened), QUICK)
        .map(|_| ());
    assert_eq!(failed, Err(Error::InitialisationFailed));
    let runtime = loader.load(&first, opener(&fake, &opened), QUICK).unwrap();
    assert_eq!(opened.load(Ordering::Relaxed), 1);
    runtime.open_session(b"init").unwrap();
    loader.shutdown();
}

#[test]
fn a_library_that_does_not_open_leaves_nothing_loaded() {
    let folder = TestFolder::new("loader-unopened");
    let (first, second) = library_files(&folder);
    let loader = Loader::new();
    let failed = loader
        .load(&first, |_| Err(Error::NotACdm("CreateCdmInstance")), QUICK)
        .map(|_| ());
    assert_eq!(failed, Err(Error::NotACdm("CreateCdmInstance")));
    assert!(loader.runtime().is_none());
    let fake = Fake::new(well_behaved());
    let opened = Arc::new(AtomicUsize::new(0));
    loader.load(&second, opener(&fake, &opened), QUICK).unwrap();
    loader.shutdown();
}

fn count(fake: &Fake, wanted: impl Fn(&Call) -> bool) -> usize {
    fake.calls().iter().filter(|call| wanted(call)).count()
}

/// A fake whose first initialisation waits at the gate, then answers as `answer` says.
fn initialising_at(gate: &Arc<Gate>, answer: Vec<Step>) -> Arc<Fake> {
    let held = Arc::clone(gate);
    let mut base = well_behaved();
    let mut first = true;
    Fake::new(move |call: &Call| match call {
        Call::Initialize { .. } if first => {
            first = false;
            let mut steps = vec![Step::Wait(Arc::clone(&held))];
            steps.extend(answer.iter().cloned());
            steps
        }
        other => base(other),
    })
}

#[test]
fn threads_loading_at_once_share_the_one_instance() {
    let folder = TestFolder::new("loader-together");
    let (first, _) = library_files(&folder);
    let gate = Arc::new(Gate::default());
    let fake = initialising_at(&gate, vec![Step::Initialized(true)]);
    let opened = Arc::new(AtomicUsize::new(0));
    let loader = Loader::new();
    thread::scope(|scope| {
        let load = || {
            loader
                .load(&first, opener(&fake, &opened), QUICK)
                .map(|runtime| runtime.open_session(b"init").map(|_| ()))
        };
        let one = scope.spawn(load);
        // The others come while the first holds the load, its CDM initialising.
        gate.wait_until_entered();
        let others: Vec<_> = (0..3).map(|_| scope.spawn(load)).collect();
        gate.open();
        assert_eq!(one.join().unwrap(), Ok(Ok(())));
        for other in others {
            assert_eq!(other.join().unwrap(), Ok(Ok(())));
        }
    });
    loader.shutdown();
    assert_eq!(opened.load(Ordering::Relaxed), 1);
    assert_eq!(count(&fake, |call| matches!(call, Call::Create { .. })), 1);
    assert_eq!(
        count(&fake, |call| matches!(call, Call::Initialize { .. })),
        1
    );
    assert_eq!(fake.calls().last(), Some(&Call::ModuleDeinitialised));
}

#[test]
fn a_shutdown_while_the_cdm_initialises_ends_the_load() {
    let folder = TestFolder::new("loader-shutdown-initialising");
    let (first, _) = library_files(&folder);
    let gate = Arc::new(Gate::default());
    // The CDM never answers its initialisation.
    let fake = initialising_at(&gate, Vec::new());
    let opened = Arc::new(AtomicUsize::new(0));
    let loader = Loader::new();
    thread::scope(|scope| {
        let loading = scope.spawn(|| {
            loader
                .load(&first, opener(&fake, &opened), QUICK)
                .map(|_| ())
        });
        gate.wait_until_entered();
        let runtime = loader.runtime().expect("the CDM thread runs");
        let shutting = scope.spawn(|| loader.shutdown());
        while runtime.shared().lock_queue().shutdown_started().is_none() {
            thread::yield_now();
        }
        let after = scope.spawn(|| {
            loader
                .load(&first, opener(&fake, &opened), QUICK)
                .map(|_| ())
        });
        gate.open();
        assert_eq!(loading.join().unwrap(), Err(Error::ShutDown));
        assert_eq!(after.join().unwrap(), Err(Error::ShutDown));
        shutting.join().unwrap();
    });
    assert_eq!(opened.load(Ordering::Relaxed), 1);
    assert_eq!(fake.calls().last(), Some(&Call::ModuleDeinitialised));
}

#[test]
fn a_shutdown_while_the_library_opens_leaves_nothing_loaded() {
    let folder = TestFolder::new("loader-shutdown-opening");
    let (first, _) = library_files(&folder);
    let fake = Fake::new(well_behaved());
    let (entered, opening) = channel();
    let (go_on, may_go_on) = channel::<()>();
    let loader = Loader::new();
    thread::scope(|scope| {
        let library = fake.library();
        let loading = scope.spawn(|| {
            let open = move |_: &Path| {
                entered.send(()).unwrap();
                may_go_on.recv().unwrap();
                Ok(library)
            };
            loader.load(&first, open, QUICK).map(|_| ())
        });
        opening.recv().unwrap();
        // Shutting down never waits for a load: no CDM thread runs yet.
        loader.shutdown();
        go_on.send(()).unwrap();
        assert_eq!(loading.join().unwrap(), Err(Error::ShutDown));
    });
    assert!(loader.runtime().is_none());
    assert!(fake.calls().is_empty(), "the CDM thread never started");
}

#[test]
fn nothing_loads_after_shutdown() {
    let folder = TestFolder::new("loader-shutdown");
    let (first, _) = library_files(&folder);
    let fake = Fake::new(well_behaved());
    let opened = Arc::new(AtomicUsize::new(0));

    let never_loaded = Loader::new();
    never_loaded.shutdown();
    let refused = never_loaded
        .load(&first, opener(&fake, &opened), QUICK)
        .map(|_| ());
    assert_eq!(refused, Err(Error::ShutDown));
    assert_eq!(opened.load(Ordering::Relaxed), 0);

    let loader = Loader::new();
    let runtime = loader.load(&first, opener(&fake, &opened), QUICK).unwrap();
    loader.shutdown();
    assert_eq!(
        loader
            .load(&first, opener(&fake, &opened), QUICK)
            .map(|_| ()),
        Err(Error::ShutDown)
    );
    assert_eq!(
        runtime.open_session(b"init").map(|_| ()),
        Err(Error::ShutDown)
    );
    assert_eq!(fake.calls().last(), Some(&Call::ModuleDeinitialised));
}

#[test]
fn the_public_load_refuses_a_file_that_is_not_a_library() {
    let folder = TestFolder::new("public-load");
    let path = folder.path().join("libwidevinecdm.so");
    std::fs::write(&path, b"plain text").unwrap();
    assert!(matches!(
        crate::Cdm::load(&path),
        Err(Error::LibraryNotOpened(_))
    ));
    assert!(matches!(
        crate::Cdm::load(folder.path()),
        Err(Error::LibraryNotOpened(_))
    ));
}

#[test]
fn the_handles_cross_threads() {
    fn send_and_sync<T: Send + Sync>() {}
    send_and_sync::<crate::Cdm>();
    send_and_sync::<crate::Session>();
    send_and_sync::<crate::Error>();
    fn copy<T: Copy>() {}
    copy::<crate::Cdm>();
}
