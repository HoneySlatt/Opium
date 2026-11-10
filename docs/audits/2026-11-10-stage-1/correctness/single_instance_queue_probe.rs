//! A probe, not a test for the tree: how the bus records a second
//! `Mpris::start` while another process holds Opium's name. Run as
//! `single_instance.rs` is, on a private bus.

use std::io::BufRead;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

use desktop::mpris::Mpris;

const NAME: &str = "org.mpris.MediaPlayer2.opium";
const FIRST: &str = "OPIUM_PROBE_FIRST";

fn bus_call(method: &str) -> Option<mpris_server::zbus::Message> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .ok()?;
    runtime.block_on(async {
        let bus = mpris_server::zbus::Connection::session().await.ok()?;
        bus.call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            method,
            &(NAME,),
        )
        .await
        .ok()
    })
}

fn say(when: &str) {
    let owner = bus_call("GetNameOwner").and_then(|m| m.body().deserialize::<String>().ok());
    let queue =
        bus_call("ListQueuedOwners").and_then(|m| m.body().deserialize::<Vec<String>>().ok());
    println!("{when}: owner {owner:?}, queue {queue:?}");
}

#[test]
fn probe() {
    if std::env::var_os(FIRST).is_some() {
        let _mpris = Mpris::start(Arc::new(|| {}));
        let _ = std::io::stdin().lock().read_line(&mut String::new());
        return;
    }
    say("before any");
    let mut first = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "probe", "--nocapture"])
        .env(FIRST, "1")
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(500));
    say("first started");
    let second = Mpris::start(Arc::new(|| {}));
    std::thread::sleep(Duration::from_millis(500));
    say("second started");
    drop(first.stdin.take());
    let _ = first.wait();
    std::thread::sleep(Duration::from_millis(300));
    say("first quit");
    drop(second);
    std::thread::sleep(Duration::from_millis(300));
    say("second dropped");
}
