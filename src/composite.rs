use crate::image::ImageData;

use std::{collections::HashMap, fmt::Debug};

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

pub trait Node: Debug {
    //fn set_setting(&mut self, setting: Setting, value: impl Into<Setting>); // TODO
    fn name(&self) -> &'static str; // TODO this is a hack
    fn execute(
        &self,
        input: HashMap<&'static str, ImageData>,
    ) -> Option<HashMap<&'static str, ImageData>>;

    fn input_source(&self, slot: &'static str) -> Option<&Port>;
    fn output_destinations(&self, slot: &'static str) -> Option<&[Port]>;

    fn connect_input(&mut self, slot: &'static str, from: Port);
    fn connect_output(&mut self, slot: &'static str, to: Port);
    fn remove_output(&mut self, slot: &'static str, to: &Port);

    fn has_connection(&self, slot: &'static str, to: &Port) -> bool {
        self.output_destinations(slot)
            .map_or(false, |destinations| destinations.contains(to))
    }
}

#[derive(Eq, PartialEq, Hash, Debug, Clone)]
pub struct Port {
    pub node_name: String,
    pub slot_name: &'static str,
}

#[derive(Debug)]
pub struct NodeGraph {
    nodes: HashMap<String, Box<dyn Node>>,
}

impl NodeGraph {
    pub fn new() -> Self {
        NodeGraph {
            nodes: HashMap::new(),
        }
    }

    pub fn add(&mut self, node: Box<dyn Node>) -> String {
        let mut i: usize = 0;
        while self.nodes.contains_key(&format_name(node.name(), i)) {
            i += 1;
        }

        let name = format_name(node.name(), i);
        self.nodes.insert(name.clone(), node);
        name
    }

    pub fn connect(&mut self, from: Port, to: Port) {
        // remove other outputs going to `to` (since an input slot can only have one source)
        for (_, node) in self.nodes.iter_mut() {
            // if `node`'s slot called `from.slot_name` has an output destination that is `to`
            if node.has_connection(from.slot_name, &to) {
                node.remove_output(from.slot_name, &to);
                break; // there should only be one
            }
        }

        // and connect the output of `from`...
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

#[derive(Debug)]
pub struct AlphaOver {
    pub mix: f32,
    input_a_source: Option<Port>,
    input_b_source: Option<Port>,
    mix_destinations: Vec<Port>,
}

impl AlphaOver {
    pub const INPUT_A: &'static str = "INPUT_A";
    pub const INPUT_B: &'static str = "INPUT_B";
    pub const OUTPUT_MIX: &'static str = "OUTPUT_MIX";
    pub const NAME: &'static str = "AlphaOver";

    pub fn new(mix: f32) -> Self {
        Self {
            mix,
            input_a_source: None,
            input_b_source: None,
            mix_destinations: Vec::new(),
        }
    }
}

impl Node for AlphaOver {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn execute(
        &self,
        mut input: HashMap<&'static str, ImageData>,
    ) -> Option<HashMap<&'static str, ImageData>> {
        let a = input.remove(Self::INPUT_A)?;
        let b = input.remove(Self::INPUT_B)?;

        let mut output = HashMap::new();
        output.insert(
            Self::OUTPUT_MIX,
            ImageData {
                data: a
                    .into_iter()
                    .zip(b.into_iter())
                    .map(|(a, b)| a * self.mix + b * (1. - self.mix))
                    .collect(),
            },
        );

        Some(output)
    }

    fn input_source(&self, slot: &'static str) -> Option<&Port> {
        match slot {
            Self::INPUT_A => self.input_a_source.as_ref(),
            Self::INPUT_B => self.input_b_source.as_ref(),
            _ => None,
        }
    }

    fn output_destinations(&self, slot: &'static str) -> Option<&[Port]> {
        match slot {
            Self::OUTPUT_MIX => Some(&self.mix_destinations),
            _ => None,
        }
    }

    fn connect_input(&mut self, slot: &'static str, from: Port) {
        match slot {
            Self::INPUT_A => self.input_a_source = Some(from),
            Self::INPUT_B => self.input_b_source = Some(from),
            _ => panic!(
                "cannot connect: no input slot on {} named {}",
                self.name(),
                slot
            ),
        }
    }

    fn connect_output(&mut self, slot: &'static str, to: Port) {
        match slot {
            Self::OUTPUT_MIX => self.mix_destinations.push(to),
            _ => panic!(
                "cannot connect: no output slot on {} named {}",
                self.name(),
                slot
            ),
        }
    }

    fn remove_output(&mut self, slot: &'static str, to: &Port) {
        match slot {
            Self::OUTPUT_MIX => self.mix_destinations.retain(|port| port != to),
            _ => panic!(
                "cannot remove: no output slot on {} named {}",
                self.name(),
                slot
            ),
        }
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
    let mut graph = NodeGraph::new();
    let ao1 = graph.add(Box::new(AlphaOver::new(1.0)));
    let ao2 = graph.add(Box::new(AlphaOver::new(0.6)));
    let ao3 = graph.add(Box::new(AlphaOver::new(0.3)));
    println!("{:#?}", graph);

    graph.connect(
        Port {
            node_name: ao1.clone(),
            slot_name: AlphaOver::OUTPUT_MIX,
        },
        Port {
            node_name: ao2.clone(),
            slot_name: AlphaOver::INPUT_A,
        },
    );
    println!("{:#?}", graph);

    graph.connect(
        Port {
            node_name: ao3.clone(),
            slot_name: AlphaOver::OUTPUT_MIX,
        },
        Port {
            node_name: ao2.clone(),
            slot_name: AlphaOver::INPUT_B,
        },
    );
    println!("{:#?}", graph);

    graph.connect(
        Port {
            node_name: ao3.clone(),
            slot_name: AlphaOver::OUTPUT_MIX,
        },
        Port {
            node_name: ao2.clone(),
            slot_name: AlphaOver::INPUT_A,
        },
    );
    println!("{:#?}", graph);
}
