#![allow(unused)]

use std::{error::Error, f32::consts::PI};
use std::fmt;
use futures::channel::oneshot::{Receiver, Sender};
use rppal::i2c::I2c;
use crate::i2c::I2CBit;
use std::time::Duration;
use std::thread::sleep;
use std::time::Instant;
use nalgebra::{Matrix1, Vector3};
use nalgebra::Matrix3;
use nalgebra::Matrix1x3;
use bincode::{config, Decode, Encode};

mod mag_registry;

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
    i2c: I2c,
    status: u8,
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

        // Prépare le I2C
        let i2c = I2c::new();
        match i2c {
            Ok(mut i2c) => {
                i2c.set_slave_address(mag_registry::HMC8553L_MAG_ADDR)?;

                // Créer l'objet et commence l'initialisation
                // NOTE : Pour obtenir les données de calibration, utiliser la partie "RAW" sur l'UI puis
                // le script : https://github.com/nliaudat/magnetometer_calibration/
                let mut mag = Self {
                    i2c: i2c,
                    status: 0x0,
                    heading: 0.0,
                    raw_x: 0.0,
                    raw_y: 0.0,
                    raw_z: 0.0,
                    mag_decl: 2.44,
                    hard_cal: Vector3::new(-135.88267489, 191.66016152, -45.84590397),
                    soft_cal: Matrix3::new(1.14291451, -0.0145877, -0.03896587, -0.0145877, 1.12288524, 0.02235676, -0.03896587, 0.02235676, 1.1820893),
                };

                // Prépare le module à être utilisé
                mag.init_module();

                Ok(mag)
            }

            Err(e) => {
                println!("[MAG] ERREUR: {}", e.to_string());
                Err("MAGINIT")?
            }
        }
    }

    /// Initialise rapidement le module avec des valeurs pré-défini
    pub fn init_module(&mut self) -> Result<(), Box<dyn Error>> {
        println!("[MAG] Initialisation ...");

        // Configuration par défaut pour le HMC8553L
        self.i2c.ecriture_word(mag_registry::HMC8553L_CONF_A, 0x10);
        self.i2c.ecriture_word(mag_registry::HMC8553L_CONF_B, 0x20);

        // Activation de la mesure continue
        self.i2c.ecriture_word(mag_registry::HMC8553L_MODE, 0x00);

        self.status |= 0x1;
        Ok(())
    }
    
    /// Récupére la valeur en X (RAW)
    pub fn get_mag_x_raw(&self) -> Result<i16, Box<dyn Error>> {
        let mag_x_h = self.i2c.lecture_word(mag_registry::HMC8553L_X_H)?;
        let mag_x_l = self.i2c.lecture_word(mag_registry::HMC8553L_X_L)?;
        Ok(((mag_x_h as i16) << 8) | mag_x_l as i16)
    }

    /// Récupére la valeur en Y
    pub fn get_mag_y_raw(&self) -> Result<i16, Box<dyn Error>> {
        let mag_y_h = self.i2c.lecture_word(mag_registry::HMC8553L_Y_H)?;
        let mag_y_l = self.i2c.lecture_word(mag_registry::HMC8553L_Y_L)?;
        Ok(((mag_y_h as i16) << 8) | mag_y_l as i16)
    }

    /// Récupére la valeur en Z
    pub fn get_mag_z_raw(&self) -> Result<i16, Box<dyn Error>> {
        let mag_z_h = self.i2c.lecture_word(mag_registry::HMC8553L_Z_H)?;
        let mag_z_l = self.i2c.lecture_word(mag_registry::HMC8553L_Z_L)?;
        Ok(((mag_z_h as i16) << 8) | mag_z_l as i16)
    }

    /// Permet de définir un feedback à partir de donnée d'un autre capteur (Déclinaison magnétique)
    pub fn feedback_set_mag_decl(&mut self, mag_decl: f32) {
        self.mag_decl = mag_decl;
    }

    /// Lecture des données
    pub fn read_values(&mut self) -> MAGData {
        MAGData {
            status: self.status,
            raw_x: self.raw_x,
            raw_y: self.raw_y,
            raw_z: self.raw_z,
            heading: self.heading
        }
    }

    /// Mets à jour les valeurs
    pub fn update(&mut self) -> Result<(), Box<dyn Error>> {
        // Récupération des données brut
        self.raw_x = self.get_mag_x_raw()? as f32;
        self.raw_y = self.get_mag_y_raw()? as f32;
        self.raw_z = self.get_mag_z_raw()? as f32;

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
}

