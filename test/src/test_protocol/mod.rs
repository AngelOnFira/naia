/// Minimal test protocol for E2E testing

use naia_shared::{Protocol, Property, Replicate};

#[derive(Replicate)]
pub struct Position {
    pub x: Property<f32>,
    pub y: Property<f32>,
}

impl Position {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x: Property::new_local(x),
            y: Property::new_local(y),
        }
    }
}

pub fn protocol() -> Protocol {
    Protocol::builder()
        .add_component::<Position>()
        .build()
}

