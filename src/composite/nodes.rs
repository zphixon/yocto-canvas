use crate::image::ImageData;

use super::{Node, Port};

use std::collections::HashMap;

macro_rules! impl_node {
    ($Name:ident; in $($INPUT:ident)*; out $($OUTPUT:ident)*; has $($prop:ident : $type_:ty),*; $exec:expr) => {
        #[allow(non_snake_case)]
        #[derive(Debug)]
        pub struct $Name {
            $(pub $prop : $type_,)*
            $($INPUT: Option<Port>,)*
            $($OUTPUT: Vec<Port>,)*
        }

        impl $Name {
            $(pub const $INPUT: &'static str = stringify!($INPUT);)*
            $(pub const $OUTPUT: &'static str = stringify!($OUTPUT);)*

            pub fn new($($prop: $type_,)*) -> $Name {
                $Name {
                    $($prop,)*
                    $($INPUT: None,)*
                    $($OUTPUT: Vec::new(),)*
                }
            }
        }

        impl Node for $Name {
            fn name(&self) -> &'static str {
                stringify!($Name)
            }

            fn execute(
                &self,
                input: HashMap<&'static str, ImageData>,
            ) -> Option<HashMap<&'static str, ImageData>> {
                $exec(self, input)
            }

            fn input_source(&self, input_slot: &'static str) -> Option<&Port> {
                match input_slot {
                    $(Self::$INPUT => self.$INPUT.as_ref(),)*
                    _ => None,
                }
            }

            fn output_destinations(&self, output_slot: &'static str) -> Option<&[Port]> {
                match output_slot {
                    $(Self::$OUTPUT => Some(&self.$OUTPUT),)*
                    _ => None,
                }
            }

            fn connect_input(&mut self, input_slot: &'static str, source_port: Port) {
                match input_slot {
                    $(Self::$INPUT => self.$INPUT = Some(source_port),)*
                    _ => panic!(
                        "cannot connect: no input slot on {} named {}",
                        self.name(),
                        input_slot
                    ),
                }
            }

            fn connect_output(&mut self, output_slot: &'static str, destination_port: Port) {
                match output_slot {
                    $(Self::$OUTPUT => self.$OUTPUT.push(destination_port),)*
                    _ => panic!(
                        "cannot connect: no output slot on {} named {}",
                        self.name(),
                        output_slot
                    ),
                }
            }

            fn remove_output(&mut self, output_slot: &'static str, destination_port: &Port) {
                match output_slot {
                    $(Self::$OUTPUT => self.$OUTPUT.retain(|port| port != destination_port),)*
                    _ => panic!(
                        "cannot remove: no output slot on {} named {}",
                        self.name(),
                        output_slot
                    ),
                }
            }
        }
    }
}

impl_node!(
    MixRgba;
    in INPUT_A INPUT_B;
    out OUTPUT_MIX;
    has mix: f32;

    |this: &MixRgba, mut input: HashMap<&'static str, ImageData>| {
        let a = input.remove(Self::INPUT_A)?;
        let b = input.remove(Self::INPUT_B)?;

        let mut output = HashMap::new();
        output.insert(
            Self::OUTPUT_MIX,
            ImageData {
                data: a
                    .into_iter()
                    .zip(b.into_iter())
                    .map(|(a, b)| a * this.mix + b * (1. - this.mix))
                    .collect(),
            },
        );

        Some(output)
    }
);
