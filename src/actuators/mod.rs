#[cfg(feature = "real-actuators")]
pub mod motor;

#[cfg(feature = "real-actuators")]
pub mod steering;

#[cfg(feature = "real-actuators")]
pub mod esc;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub(crate) struct Control {
    pub steer: f64,
    pub speed: f64,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Switch {
    pub esc: bool,
    pub reload: bool,
}

impl Switch {
    pub (crate) fn empty() -> Switch {
        Switch {
            esc: false,
            reload: false
        }
    }
}