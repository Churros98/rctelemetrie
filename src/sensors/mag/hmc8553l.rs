#![allow(unused)]

use std::{error::Error, f32::consts::PI};
use std::fmt;
use anyhow::anyhow;
use rppal::i2c::I2c;
use crate::i2c::I2CBit;
use crate::sensors::mag::registry;
use std::time::Duration;
use std::thread::sleep;
use std::time::Instant;
use nalgebra::{Matrix1, Vector3};
use nalgebra::Matrix3;
use nalgebra::Matrix1x3;

pub struct HMC8553L {
    mag_decl: f32,
    hard_cal: Vector3<f32>,
    soft_cal: Matrix3<f32>,
    i2c: I2c,
}

impl HMC8553L {
    /// Constructeur
    pub fn new() -> anyhow::Result<Self> {
        println!("[HMC8554L] Connexion I2C ...");

        // Prépare le I2C
        let i2c = I2c::new();
        match i2c {
            Ok(mut i2c) => {
                i2c.set_slave_address(registry::HMC8553L_MAG_ADDR)?;

                // Créer l'objet et commence l'initialisation
                // NOTE : Pour obtenir les données de calibration, utiliser la partie "RAW" sur l'UI puis
                // le script : https://github.com/nliaudat/magnetometer_calibration/
                let mut mag = Self {
                    i2c: i2c,
                    mag_decl: 2.44,
                    hard_cal: Vector3::new(-135.88267489, 191.66016152, -45.84590397),
                    soft_cal: Matrix3::new(1.14291451, -0.0145877, -0.03896587, -0.0145877, 1.12288524, 0.02235676, -0.03896587, 0.02235676, 1.1820893),
                };

                // Prépare le module à être utilisé
                mag.init_module();

                Ok(mag)
            }

            Err(e) => {
                println!("[HMC8553L] ERREUR: {}", e.to_string());
                Err(anyhow::anyhow!(e))
            }
        }
    }

    /// Initialise rapidement le module avec des valeurs pré-défini
    fn init_module(&mut self) -> anyhow::Result<()> {
        println!("[HMC8554L] Initialisation ...");

        // Configuration par défaut pour le HMC8553L
        self.i2c.ecriture_word(registry::HMC8553L_CONF_A, 0x10)?;
        self.i2c.ecriture_word(registry::HMC8553L_CONF_B, 0x20)?;

        // Activation de la mesure continue
        self.i2c.ecriture_word(registry::HMC8553L_MODE, 0x00)?;

        Ok(())
    }

    /// Récupére la valeur en X (RAW)
    fn get_mag_x_raw(&self) -> anyhow::Result<i16> {
        let mag_x_h = self.i2c.lecture_word(registry::HMC8553L_X_H)?;
        let mag_x_l = self.i2c.lecture_word(registry::HMC8553L_X_L)?;
        Ok(((mag_x_h as i16) << 8) | mag_x_l as i16)
    }

    /// Récupére la valeur en Y
    fn get_mag_y_raw(&self) -> anyhow::Result<i16> {
        let mag_y_h = self.i2c.lecture_word(registry::HMC8553L_Y_H)?;
        let mag_y_l = self.i2c.lecture_word(registry::HMC8553L_Y_L)?;
        Ok(((mag_y_h as i16) << 8) | mag_y_l as i16)
    }

    /// Récupére la valeur en Z
    fn get_mag_z_raw(&self) -> anyhow::Result<i16> {
        let mag_z_h = self.i2c.lecture_word(registry::HMC8553L_Z_H)?;
        let mag_z_l = self.i2c.lecture_word(registry::HMC8553L_Z_L)?;
        Ok(((mag_z_h as i16) << 8) | mag_z_l as i16)
    }

    /// Récupére les données raw
    pub fn get_mag_axes_raw(&self) -> anyhow::Result<Vector3<i16>> {
        let raw_x = self.get_mag_x_raw()?;
        let raw_y = self.get_mag_y_raw()?;
        let raw_z = self.get_mag_z_raw()?;

        Ok(Vector3::new(raw_x, raw_y, raw_z))
    }

    /// Récupére le heading
    pub fn get_heading(&self) -> anyhow::Result<f32> {
        // Récupération des données brut
        let raw_x = self.get_mag_x_raw()? as f32;
        let raw_y = self.get_mag_y_raw()? as f32;
        let raw_z = self.get_mag_z_raw()? as f32;

        // Correction "Hard Iron" & "Soft Iron"
        println!("{},{},{}", raw_x, raw_y, raw_z);
        let hard_mag  = Matrix1x3::new(raw_x - self.hard_cal.x, raw_y - self.hard_cal.y, raw_z - self.hard_cal.z);
        let corrected_mag = hard_mag * self.soft_cal;

        // Calcul du heading, prend en compte la déclinaison magnétique
        let mut heading = -((corrected_mag.x.atan2(corrected_mag.y) * (180.0 / PI)) + self.mag_decl);

        Ok(heading)
    }
}
