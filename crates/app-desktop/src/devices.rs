//! Music Connect on the desktop: this computer on the network, for the
//! user's phone to see what plays here, control it, and hand music over.
//! Its events come back to the app each frame, as MPRIS's actions do.

use std::sync::mpsc::Receiver;

use connect::{Device, DeviceKind, Event, Node, Peer};

pub struct Devices {
    pub node: Node,
    events: Receiver<Event>,
    /// The devices seen or paired, as last told.
    pub peers: Vec<Peer>,
    /// A device asking to pair, and the code to show it.
    pub pairing: Option<(Device, String)>,
    /// The queue's revision last told, so it goes out once it changes.
    pub upcoming_told: Option<u64>,
}

impl Devices {
    /// Go on the network under the computer's name; `ctx` is woken when
    /// something is heard.
    pub fn start(ctx: egui::Context) -> Self {
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
            move |event| {
                let _ = tx.send(event);
                ctx.request_repaint();
            },
        );
        let peers = node.peers();
        Self {
            node,
            events,
            peers,
            pairing: None,
            upcoming_told: None,
        }
    }

    pub fn next_event(&self) -> Option<Event> {
        self.events.try_recv().ok()
    }
}
