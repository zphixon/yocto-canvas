use crate::image::ImageData;

use std::{collections::HashMap, fmt::Debug};

pub mod nodes;

// generate a new node name
fn format_name(s: &str, i: usize) -> String {
    format!(
        "{}{}",
        s,
        if i == 0 {
            String::new()
        } else {
            format!("{}", i)
        }
    )
}

// TODO proc macro???? that would be sick
pub trait Node: Debug {
    //fn set_setting(&mut self, setting: Setting, value: impl Into<Setting>); // TODO
    /// Get the name of the node.
    ///
    /// Used to automatically generate names for new nodes in the graph.
    fn name(&self) -> &'static str; // TODO this is a hack

    /// TODO Execute the node.
    ///
    /// Meant to only be called by NodeGraph.
    fn execute(
        &self,
        input: HashMap<&'static str, ImageData>,
    ) -> Option<HashMap<&'static str, ImageData>>;

    /// Get the node and output slot connected to the input slot.
    fn input_source(&self, input_slot: &'static str) -> Option<&Port>;

    /// Get the destination ports of the output slot.
    fn output_destinations(&self, output_slot: &'static str) -> Option<&[Port]>;

    /// Connect the input slot to the source port. Must replace the connection.
    ///
    /// Data flows from `source_port.node_name.output_port_name` to `self.input_slot`.
    fn connect_input(&mut self, input_slot: &'static str, source_port: Port);

    /// Connect the output slot to the destination port.
    ///
    /// Data flows from `self.output_slot` to `destination_port.node_name.input_port_name`.
    fn connect_output(&mut self, output_slot: &'static str, destination_port: Port);

    /// Remove the destination port from the output slot.
    ///
    /// Data will no longer flow from `self.output_slot` to `destination_port.node_name.input_port_name`.
    fn remove_output(&mut self, output_slot: &'static str, destination_port: &Port);

    /// Check if the node has a connection from `self.output_slot` to `destination_port.node_name.input_port_name`.
    fn has_connection(&self, output_slot: &'static str, destination_port: &Port) -> bool {
        self.output_destinations(output_slot)
            .map_or(false, |destinations| {
                destinations.contains(destination_port)
            })
    }
}

/// Represents a single end of a node graph connection
#[derive(Eq, PartialEq, Hash, Debug, Clone)]
pub struct Port {
    pub node_name: String,
    pub slot_name: &'static str,
}

/// Contains the full node graph as an intrusive digraph
#[derive(Debug)]
pub struct NodeGraph {
    nodes: HashMap<String, Box<dyn Node>>,
}

// TODO check for cycles
impl NodeGraph {
    /// Create a new node graph.
    pub fn new() -> Self {
        NodeGraph {
            nodes: HashMap::new(),
        }
    }

    /// Add a node to the graph. Returns the name of the node.
    ///
    /// Use `connect` to add connections to the node.
    pub fn add(&mut self, node: Box<dyn Node>) -> String {
        let mut i: usize = 0;
        while self.nodes.contains_key(&format_name(node.name(), i)) {
            i += 1;
        }

        let name = format_name(node.name(), i);
        self.nodes.insert(name.clone(), node);
        name
    }

    /// Connect one node to another node.
    ///
    /// The input port on `to` is cleared of its connection, if it exists. The corresponding port on
    /// the output node of the node connected to this node is also removed. The ports are then
    /// connected.
    pub fn connect(&mut self, from: Port, to: Port) {
        // remove other outputs going to `to` (since an input slot can only have one source)
        for (_, node) in self.nodes.iter_mut() {
            // if `node`'s slot called `from.slot_name` has an output destination that is `to`
            if node.has_connection(from.slot_name, &to) {
                node.remove_output(from.slot_name, &to);
                break; // there should only be one
            }
        }

        // and then connect the output of `from`...
        self.nodes
            .get_mut(&from.node_name)
            .unwrap()
            .connect_output(from.slot_name, to.clone());

        // to the input of `to`
        self.nodes
            .get_mut(&to.node_name)
            .unwrap()
            .connect_input(to.slot_name, from.clone());
    }
}

#[test]
fn format_name_correct() {
    assert_eq!(String::from("a"), format_name("a", 0));
    assert_eq!(String::from("a1"), format_name("a", 1));
    assert_eq!(String::from("a2"), format_name("a", 2));
}

#[test]
fn node_graph_connect() {
    use nodes::MixRgba;

    let mut graph = NodeGraph::new();
    let ao1 = graph.add(Box::new(MixRgba::new(1.0)));
    let ao2 = graph.add(Box::new(MixRgba::new(0.6)));
    let ao3 = graph.add(Box::new(MixRgba::new(0.3)));
    println!("{:#?}", graph);

    graph.connect(
        Port {
            node_name: ao1.clone(),
            slot_name: MixRgba::OUTPUT_MIX,
        },
        Port {
            node_name: ao2.clone(),
            slot_name: MixRgba::INPUT_A,
        },
    );
    println!("{:#?}", graph);

    graph.connect(
        Port {
            node_name: ao3.clone(),
            slot_name: MixRgba::OUTPUT_MIX,
        },
        Port {
            node_name: ao2.clone(),
            slot_name: MixRgba::INPUT_B,
        },
    );
    println!("{:#?}", graph);

    graph.connect(
        Port {
            node_name: ao3.clone(),
            slot_name: MixRgba::OUTPUT_MIX,
        },
        Port {
            node_name: ao2.clone(),
            slot_name: MixRgba::INPUT_A,
        },
    );
    println!("{:#?}", graph);

    panic!("ok");
}
