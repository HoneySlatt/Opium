//! What else is on the PipeWire graph around our stream: the device it plays
//! into, the other programs mixed into that device, and the device's software
//! volume. All three decide whether the music reaches the card unchanged.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use pipewire as pw;
use pw::registry::GlobalObject;
use pw::spa;
use pw::spa::pod::{Value, ValueArray, deserialize::PodDeserializer};
use pw::types::ObjectType;

use crate::Mixing;

const SINK: &str = "Audio/Sink";
const PLAYBACK: &str = "Stream/Output/Audio";

#[derive(Default)]
struct NodeEntry {
    class: String,
    /// What a person would call it: the device's description, the program's name.
    label: String,
    running: bool,
    /// Linear amplitude actually applied in software, when not unity.
    soft_volume: Option<f32>,
    muted: bool,
}

#[derive(Default)]
pub(crate) struct Graph {
    nodes: HashMap<u32, NodeEntry>,
    /// Link id → (output node, input node).
    links: HashMap<u32, (u32, u32)>,
    /// Bound nodes and their listeners, kept alive while the node exists.
    proxies: HashMap<u32, (pw::node::Node, pw::node::NodeListener)>,
}

impl Graph {
    /// The view from our stream's node.
    pub fn summary(&self, ours: u32) -> Mixing {
        let Some(device) = self
            .links
            .values()
            .find(|&&(out, _)| out == ours)
            .map(|&(_, input)| input)
        else {
            return Mixing::default();
        };
        let entry = self.nodes.get(&device);
        let mut others: Vec<String> = self
            .links
            .values()
            .filter(|&&(out, input)| input == device && out != ours)
            .filter_map(|(out, _)| self.nodes.get(out))
            .filter(|n| n.class == PLAYBACK && n.running)
            .map(|n| n.label.clone())
            .collect();
        others.sort();
        others.dedup();
        Mixing {
            device: entry.map(|e| e.label.clone()),
            others,
            device_volume: entry.and_then(|e| e.soft_volume),
            device_muted: entry.is_some_and(|e| e.muted),
        }
    }
}

/// Follow the registry: nodes that play or are played into, and the links
/// between them. The returned listener must be kept alive.
pub(crate) fn watch(
    registry: &pw::registry::RegistryRc,
) -> (Rc<RefCell<Graph>>, pw::registry::Listener) {
    let graph = Rc::new(RefCell::new(Graph::default()));
    let listener = registry
        .add_listener_local()
        .global({
            let graph = graph.clone();
            let registry = registry.downgrade();
            move |object| {
                let Some(registry) = registry.upgrade() else {
                    return;
                };
                match object.type_ {
                    ObjectType::Node => add_node(&graph, &registry, object),
                    ObjectType::Link => add_link(&graph, object),
                    _ => {}
                }
            }
        })
        .global_remove({
            let graph = graph.clone();
            move |id| {
                let mut g = graph.borrow_mut();
                g.nodes.remove(&id);
                g.links.remove(&id);
                g.proxies.remove(&id);
            }
        })
        .register();
    (graph, listener)
}

fn prop<'a>(object: &'a GlobalObject<&spa::utils::dict::DictRef>, key: &str) -> Option<&'a str> {
    object.props.and_then(|p| p.get(key))
}

fn add_link(graph: &Rc<RefCell<Graph>>, object: &GlobalObject<&spa::utils::dict::DictRef>) {
    let node = |key| prop(object, key).and_then(|v| v.parse::<u32>().ok());
    if let (Some(out), Some(input)) = (node("link.output.node"), node("link.input.node")) {
        graph.borrow_mut().links.insert(object.id, (out, input));
    }
}

fn add_node(
    graph: &Rc<RefCell<Graph>>,
    registry: &pw::registry::RegistryRc,
    object: &GlobalObject<&spa::utils::dict::DictRef>,
) {
    let class = prop(object, "media.class").unwrap_or_default();
    if class != SINK && class != PLAYBACK {
        return;
    }
    let label = ["node.description", "application.name", "node.name"]
        .into_iter()
        .find_map(|key| prop(object, key))
        .unwrap_or("Unknown")
        .to_owned();
    let Ok(node) = registry.bind::<pw::node::Node, _>(object) else {
        return;
    };
    let id = object.id;
    let listener = node
        .add_listener_local()
        .info({
            let graph = Rc::downgrade(graph);
            move |info| {
                if let Some(graph) = graph.upgrade()
                    && let Some(entry) = graph.borrow_mut().nodes.get_mut(&id)
                {
                    entry.running = matches!(info.state(), pw::node::NodeState::Running);
                }
            }
        })
        .param({
            let graph = Rc::downgrade(graph);
            move |_, _, _, _, pod| {
                if let (Some(graph), Some(pod)) = (graph.upgrade(), pod)
                    && let Some(entry) = graph.borrow_mut().nodes.get_mut(&id)
                {
                    read_volume(entry, pod);
                }
            }
        })
        .register();
    if class == SINK {
        node.subscribe_params(&[spa::param::ParamType::Props]);
    }
    let mut g = graph.borrow_mut();
    g.nodes.insert(
        id,
        NodeEntry {
            class: class.to_owned(),
            label,
            ..NodeEntry::default()
        },
    );
    g.proxies.insert(id, (node, listener));
}

/// The volume PipeWire applies in software: `softVolumes` when the device also
/// has a hardware mixer (which leaves the samples alone), else `channelVolumes`.
fn read_volume(entry: &mut NodeEntry, pod: &spa::pod::Pod) {
    let Ok((_, Value::Object(object))) = PodDeserializer::deserialize_any_from(pod.as_bytes())
    else {
        return;
    };
    let (mut soft, mut channel, mut soft_mute, mut mute) = (None, None, None, None);
    for property in object.properties {
        match (property.key, property.value) {
            (spa::sys::SPA_PROP_softVolumes, Value::ValueArray(ValueArray::Float(v))) => {
                soft = v.into_iter().reduce(f32::max);
            }
            (spa::sys::SPA_PROP_channelVolumes, Value::ValueArray(ValueArray::Float(v))) => {
                channel = v.into_iter().reduce(f32::max);
            }
            (spa::sys::SPA_PROP_softMute, Value::Bool(b)) => soft_mute = Some(b),
            (spa::sys::SPA_PROP_mute, Value::Bool(b)) => mute = Some(b),
            _ => {}
        }
    }
    let volume = soft.or(channel);
    if volume.is_some() {
        entry.soft_volume = volume.filter(|&v| (v - 1.0).abs() > 1e-4);
    }
    if let Some(m) = soft_mute.or(mute) {
        entry.muted = m;
    }
}
