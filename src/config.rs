use nalgebra::{Matrix3, Vector3};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub(crate) kp: f64,
    pub(crate) ki: f64,
    pub(crate) kd: f64,
    pub(crate) mag_decl: f32,
    pub(crate) hard_cal: Vector3<f32>,
    pub(crate) soft_cal: Matrix3<f32>,
}

impl Config {
    pub fn new() -> Self {
        let config = Config { 
            kp: 1.0,
            ki: 1.0,
            kd: 1.0,
            mag_decl: 2.44,
            hard_cal: Vector3::new(569.68423502, 246.04798002, -166.97661026),
            soft_cal: Matrix3::new(
                1.08480289,
                -0.04408938,
                0.06070396,
                -0.04408938,
                1.03604676,
                0.09354455,
                0.06070396,
                0.09354455,
                0.99634431,
            ),
        };

        config
    }
}