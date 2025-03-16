#![allow(unused)]

use crate::config::Config;
use crate::i2c::I2CBit;
use crate::sensors::mag::registry;
use anyhow::anyhow;
use nalgebra::Matrix1x3;
use nalgebra::Matrix3;
use nalgebra::{Matrix1, Vector3};
use rppal::i2c::I2c;
use std::fmt;
use std::thread::sleep;
use std::time::Duration;
use std::time::Instant;
use std::{error::Error, f32::consts::PI};

pub (crate) struct HMC8553L {
    mag_decl: f32,
    hard_cal: Vector3<f32>,
    soft_cal: Matrix3<f32>,
}

impl HMC8553L {
    /// Constructeur
    pub (crate) fn new(i2c: &mut I2c, config: Config) -> anyhow::Result<Self> {
        // Créer l'objet et commence l'initialisation
        // NOTE : Pour obtenir les données de calibration, utiliser la partie "RAW" sur l'UI puis
        // le script : https://github.com/nliaudat/magnetometer_calibration/
        let mut mag = Self {
            mag_decl: config.mag_decl,
            hard_cal: config.hard_cal,
            soft_cal: config.soft_cal,
        };

        // Prépare le module à être utilisé
        mag.set_slave(i2c)?;
        mag.init_module(i2c)?;

        Ok(mag)
    }

    fn set_slave(&self, i2c: &mut I2c) -> anyhow::Result<()> {
        i2c.set_slave_address(registry::HMC8553L_MAG_ADDR);
        Ok(())
    }

    /// Initialise rapidement le module avec des valeurs pré-défini
    fn init_module(&mut self, i2c: &mut I2c) -> anyhow::Result<()> {
        println!("[HMC8554L] Initialisation (CONF A) ...");

        // Configuration par défaut pour le HMC8553L
        i2c.ecriture_word(registry::HMC8553L_CONF_A, 0x10)?;

        println!("[HMC8554L] Initialisation (CONF B) ...");
        i2c.ecriture_word(registry::HMC8553L_CONF_B, 0x20)?;

        // Activation de la mesure continue
        println!("[HMC8554L] Initialisation (MODE) ...");
        i2c.ecriture_word(registry::HMC8553L_MODE, 0x00)?;

        println!("[HMC8554L] Fin d'initialisation.");

        Ok(())
    }

    /// Récupére la valeur en X (RAW)
    fn get_mag_x_raw(&self, i2c: &mut I2c) -> anyhow::Result<i16> {
        let mag_x_h = i2c.lecture_word(registry::HMC8553L_X_H)?;
        let mag_x_l = i2c.lecture_word(registry::HMC8553L_X_L)?;
        Ok(((mag_x_h as i16) << 8) | mag_x_l as i16)
    }

    /// Récupére la valeur en Y
    fn get_mag_y_raw(&self, i2c: &mut I2c) -> anyhow::Result<i16> {
        let mag_y_h = i2c.lecture_word(registry::HMC8553L_Y_H)?;
        let mag_y_l = i2c.lecture_word(registry::HMC8553L_Y_L)?;
        Ok(((mag_y_h as i16) << 8) | mag_y_l as i16)
    }

    /// Récupére la valeur en Z
    fn get_mag_z_raw(&self, i2c: &mut I2c) -> anyhow::Result<i16> {
        let mag_z_h = i2c.lecture_word(registry::HMC8553L_Z_H)?;
        let mag_z_l = i2c.lecture_word(registry::HMC8553L_Z_L)?;
        Ok(((mag_z_h as i16) << 8) | mag_z_l as i16)
    }

    /// Récupére les données raw
    pub (crate) fn get_mag_axes_raw(&self, i2c: &mut I2c) -> anyhow::Result<Vector3<i16>> {
        // Défini mon capteur sur le bus I2C
        self.set_slave(i2c)?;

        // Récupére les valeurs RAW.
        let raw_x = self.get_mag_x_raw(i2c)?;
        let raw_y = self.get_mag_y_raw(i2c)?;
        let raw_z = self.get_mag_z_raw(i2c)?;

        Ok(Vector3::new(raw_x, raw_y, raw_z))
    }

    /// Récupére le heading
    pub (crate) fn get_heading(&self, i2c: &mut I2c) -> anyhow::Result<f32> {
        // Défini mon capteur sur le bus I2C
        self.set_slave(i2c)?;

        // Récupération des données brut
        let raw_x = self.get_mag_x_raw(i2c)? as f32;
        let raw_y = self.get_mag_y_raw(i2c)? as f32;
        let raw_z = self.get_mag_z_raw(i2c)? as f32;

        // Correction "Hard Iron" & "Soft Iron"
        let hard_mag = Matrix1x3::new(
            raw_x - self.hard_cal.x,
            raw_y - self.hard_cal.y,
            raw_z - self.hard_cal.z,
        );
        let corrected_mag = hard_mag * self.soft_cal;

        // Calcul du heading, prend en compte la déclinaison magnétique
        let mut heading = (-((corrected_mag.x.atan2(corrected_mag.y) * (180.0 / PI)) + self.mag_decl)) + 180.0;
        if (heading < 0.0) {
            heading = heading + 360.0;
        }

        Ok(heading)
    }
}
