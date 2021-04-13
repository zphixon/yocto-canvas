use crate::image::ImageData;

use super::{Node, Port};

use std::collections::HashMap;

#[derive(Debug)]
pub struct MixRgba {
    pub mix: f32,
    input_a_source: Option<Port>,
    input_b_source: Option<Port>,
    mix_destinations: Vec<Port>,
}

impl MixRgba {
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

impl Node for MixRgba {
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

    fn input_source(&self, input_slot: &'static str) -> Option<&Port> {
        match input_slot {
            Self::INPUT_A => self.input_a_source.as_ref(),
            Self::INPUT_B => self.input_b_source.as_ref(),
            _ => None,
        }
    }

    fn output_destinations(&self, output_slot: &'static str) -> Option<&[Port]> {
        match output_slot {
            Self::OUTPUT_MIX => Some(&self.mix_destinations),
            _ => None,
        }
    }

    fn connect_input(&mut self, input_slot: &'static str, source_port: Port) {
        match input_slot {
            Self::INPUT_A => self.input_a_source = Some(source_port),
            Self::INPUT_B => self.input_b_source = Some(source_port),
            _ => panic!(
                "cannot connect: no input slot on {} named {}",
                self.name(),
                input_slot
            ),
        }
    }

    fn connect_output(&mut self, output_slot: &'static str, destination_port: Port) {
        match output_slot {
            Self::OUTPUT_MIX => self.mix_destinations.push(destination_port),
            _ => panic!(
                "cannot connect: no output slot on {} named {}",
                self.name(),
                output_slot
            ),
        }
    }

    fn remove_output(&mut self, output_slot: &'static str, destination_port: &Port) {
        match output_slot {
            Self::OUTPUT_MIX => self
                .mix_destinations
                .retain(|port| port != destination_port),
            _ => panic!(
                "cannot remove: no output slot on {} named {}",
                self.name(),
                output_slot
            ),
        }
    }
}
