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

use crate::sensors::mag::mag_registry;
use crate::sensors::mag::definition;
use crate::sensors::mag::MagSensor;

pub struct HMC8553L {
    i2c: I2c,
    status: u8,
}

impl HMC8553L {
    /// Constructeur
    pub fn new() -> Result<Self, Box<dyn Error>> {
        println!("[HMC8554L] Initialisation ...");

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
                };

                // Prépare le module à être utilisé
                mag.init_module();

                Ok(mag)
            }

            Err(e) => {
                println!("[HMC8553L] ERREUR: {}", e.to_string());
                Err("HMC8553LINIT")?
            }
        }
    }

    /// Initialise rapidement le module avec des valeurs pré-défini
    fn init_module(&mut self) -> Result<(), Box<dyn Error>> {
        println!("[HMC8554L] Initialisation ...");

        // Configuration par défaut pour le HMC8553L
        self.i2c.ecriture_word(mag_registry::HMC8553L_CONF_A, 0x10);
        self.i2c.ecriture_word(mag_registry::HMC8553L_CONF_B, 0x20);

        // Activation de la mesure continue
        self.i2c.ecriture_word(mag_registry::HMC8553L_MODE, 0x00);

        self.status |= 0x1;
        Ok(())
    }
}

impl MagSensor for HMC8553L {
    /// Récupére la valeur en X (RAW)
    fn get_mag_x_raw(&self) -> Result<i16, Box<dyn Error>> {
        let mag_x_h = self.i2c.lecture_word(mag_registry::HMC8553L_X_H)?;
        let mag_x_l = self.i2c.lecture_word(mag_registry::HMC8553L_X_L)?;
        Ok(((mag_x_h as i16) << 8) | mag_x_l as i16)
    }

    /// Récupére la valeur en Y
    fn get_mag_y_raw(&self) -> Result<i16, Box<dyn Error>> {
        let mag_y_h = self.i2c.lecture_word(mag_registry::HMC8553L_Y_H)?;
        let mag_y_l = self.i2c.lecture_word(mag_registry::HMC8553L_Y_L)?;
        Ok(((mag_y_h as i16) << 8) | mag_y_l as i16)
    }

    /// Récupére la valeur en Z
    fn get_mag_z_raw(&self) -> Result<i16, Box<dyn Error>> {
        let mag_z_h = self.i2c.lecture_word(mag_registry::HMC8553L_Z_H)?;
        let mag_z_l = self.i2c.lecture_word(mag_registry::HMC8553L_Z_L)?;
        Ok(((mag_z_h as i16) << 8) | mag_z_l as i16)
    }

    /// Permet de récupérer le status du capteur
    fn get_status(&self) -> u8 {
        self.status
    }
}
