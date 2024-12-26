#[cfg(feature = "real-actuators")]
pub mod motor;

#[cfg(feature = "real-actuators")]
pub mod steering;

#[cfg(feature = "real-actuators")]
pub mod switch;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub(crate) struct Control {
    pub steer: f64,
    pub speed: f64,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Switch {
    pub esc: bool,
}