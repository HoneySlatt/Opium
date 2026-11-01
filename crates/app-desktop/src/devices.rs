//! Music Connect on the desktop: this computer on the network, for the
//! user's phone to see what plays here, control it, and hand music over.
//! Its events come back to the app each frame, as MPRIS's actions do.

use std::sync::mpsc::Receiver;

use connect::{Device, DeviceKind, Event, Node, Peer};

pub struct Devices {
    pub node: Node,
    events: Receiver<Event>,
    /// The devices seen or paired, as last told, and when that was.
    pub peers: Vec<Peer>,
    pub read_at: std::time::Instant,
    /// Pairing with a device that shows a code: it, and the code typed so far.
    pub asking: Option<(Device, String)>,
    /// A device asking to pair, and the code to show it.
    pub pairing: Option<(Device, String)>,
    /// The queue's revision last told, so it goes out once it changes.
    pub upcoming_told: Option<u64>,
}

impl Devices {
    /// Go on the network under the computer's name; `ctx` is woken when
    /// something is heard.
    pub fn start(ctx: egui::Context, kept: synced::Kept, relay: Option<String>) -> Self {
        let name = std::fs::read_to_string("/proc/sys/kernel/hostname")
            .ok()
            .map(|n| n.trim().to_owned())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| "Computer".to_owned());
        let (tx, events) = std::sync::mpsc::channel();
        let node = Node::start(
            name,
            DeviceKind::Computer,
            &crate::settings::dir(),
            kept,
            relay,
            move |event| {
                let _ = tx.send(event);
                ctx.request_repaint();
            },
        );
        // Plugged in: always reachable away from home.
        node.set_away(true);
        let peers = node.peers();
        Self {
            node,
            events,
            peers,
            read_at: std::time::Instant::now(),
            asking: None,
            pairing: None,
            upcoming_told: None,
        }
    }

    pub fn read(&mut self) {
        self.peers = self.node.peers();
        self.read_at = std::time::Instant::now();
    }

    pub fn next_event(&self) -> Option<Event> {
        self.events.try_recv().ok()
    }
}
