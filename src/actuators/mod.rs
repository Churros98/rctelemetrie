#[cfg(feature = "real-actuators")]
pub mod motor;

#[cfg(feature = "real-actuators")]
pub mod steering;

use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct Control {
    pub steer: f64,
    pub speed: f64,
}