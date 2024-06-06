pub mod motor;
pub mod steering;

use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct Control {
    pub steer: f64,
    pub speed: f64,
}