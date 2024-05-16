#![allow(unused)]

use std::task::Poll;
use std::{error::Error, f32::consts::PI};
use std::fmt;
use futures::channel::oneshot::{Receiver, Sender};
use tokio_stream::Stream;
use rppal::i2c::I2c;
use crate::i2c::I2CBit;
use crate::sensors::mag::hmc8553l::HMC8553L;
use crate::sensors::mag::qmc5883l::QMC5883L;
use std::time::Duration;
use std::thread::sleep;
use std::time::Instant;
use nalgebra::{Matrix1, Vector3};
use nalgebra::Matrix3;
use nalgebra::Matrix1x3;
use bincode::{config, Decode, Encode};
use std::sync::Arc;

use self::definition::MagSensor;

mod definition;
mod mag_registry;
mod hmc8553l;
mod qmc5883l;

/// Structure de données issus du capteur magnétique 3 axes
#[derive(Encode, Decode, Clone, Debug, Copy)]
pub struct MAGData {
    pub status: u8,
    pub raw_x: f32,
    pub raw_y: f32,
    pub raw_z: f32,
    pub heading: f32,
}

impl fmt::Display for MAGData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Heading: {}", self.heading)
    }
}

pub struct MAG {
    sensor: Box<dyn MagSensor + Send>,
    heading: f32,
    raw_x: f32,
    raw_y: f32,
    raw_z: f32,
    mag_decl: f32,
    hard_cal: Vector3<f32>,
    soft_cal: Matrix3<f32>,
}

impl MAG {
    /// Constructeur
    pub fn new() -> Result<Self, Box<dyn Error>> {
        println!("[MAG] Initialisation ...");

        let sensor: Box<dyn MagSensor + Send> = Box::new(HMC8553L::new()?) as Box<dyn MagSensor + Send>;

        let mut mag: MAG = Self {
            sensor: sensor,
            heading: 0.0,
            raw_x: 0.0,
            raw_y: 0.0,
            raw_z: 0.0,
            mag_decl: 2.44,
            hard_cal: Vector3::new(-135.88267489, 191.66016152, -45.84590397),
            soft_cal: Matrix3::new(1.14291451, -0.0145877, -0.03896587, -0.0145877, 1.12288524, 0.02235676, -0.03896587, 0.02235676, 1.1820893),
        };

        Ok(mag)
    }

    /// Lecture des données
    pub fn to_data(&mut self) -> MAGData {
        MAGData {
            status: self.sensor.get_status(),
            raw_x: self.raw_x,
            raw_y: self.raw_y,
            raw_z: self.raw_z,
            heading: self.heading
        }
    }

    /// Lis et mets à jour les valeurs
    pub fn update(&mut self) -> Result<(), Box<dyn Error>> {
        // Récupération des données brut
        self.raw_x = self.sensor.get_mag_x_raw()? as f32;
        self.raw_y = self.sensor.get_mag_y_raw()? as f32;
        self.raw_z = self.sensor.get_mag_z_raw()? as f32;

        // Correction "Hard Iron" & "Soft Iron"
        let hard_mag  = Matrix1x3::new(self.raw_x - self.hard_cal.x, self.raw_y - self.hard_cal.y, self.raw_z - self.hard_cal.z);
        let corrected_mag = hard_mag * self.soft_cal;

        // Calcul du heading, prend en compte la déclinaison magnétique
        let mut heading = -((corrected_mag.x.atan2(corrected_mag.y) * (180.0 / PI)) + self.mag_decl);

        if (heading < 0.0) {
            heading = heading + 360.0;
        }

        self.heading = heading;

        Ok(())
    }

    /// Retourne des données vide
    pub fn empty() -> MAGData {
        MAGData {
            status: 0xFF,
            raw_x: 0.0,
            raw_y: 0.0,
            raw_z: 0.0,            
            heading: 0.0,
        }
    }

    /// Permet de définir un feedback à partir de donnée d'un autre capteur (Déclinaison magnétique)
    pub fn feedback_set_mag_decl(&mut self, mag_decl: f32) {
        self.mag_decl = mag_decl;
    }
}
