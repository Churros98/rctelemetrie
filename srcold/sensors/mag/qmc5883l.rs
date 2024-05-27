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

pub struct QMC5883L {
    i2c: I2c,
    status: u8,
}

impl QMC5883L {
    /// Constructeur
    pub fn new() -> Result<Self, Box<dyn Error>> {
        println!("[QMC5883L] Initialisation ...");

        // Prépare le I2C
        let i2c = I2c::new();
        match i2c {
            Ok(mut i2c) => {
                i2c.set_slave_address(mag_registry::QMC5883L_MAG_ADDR)?;

                // Créer l'objet et commence l'initialisation
                let mut mag = Self {
                    i2c: i2c,
                    status: 0x0,
                };

                // Prépare le module à être utilisé
                mag.init_module();

                Ok(mag)
            }

            Err(e) => {
                println!("[QMC5883L] ERREUR: {}", e.to_string());
                Err("QMC5883LINIT")?
            }
        }
    }

    /// Initialise rapidement le module avec des valeurs pré-défini
    fn init_module(&mut self) -> Result<(), Box<dyn Error>> {
        println!("[QMC5883L] Initialisation ...");
        self.set_setreset(true)?;
        self.set_mode(1)?;
        self.set_output_rate(0)?;
        self.set_scale(0)?;
        self.set_osr(0)?;
        self.status |= 0x1;
        Ok(())
    }
    
    /// Récupére le "Chip ID"
    fn get_chip_id(&self) -> Result<u8, Box<dyn Error>> {
        self.i2c.lecture_word(mag_registry::QMC5883L_CHIP_ID)
    }

    /// Vérifie si les données sont disponible
    fn is_data_ready(&self) -> Result<bool, Box<dyn Error>> {
        self.i2c.lecture_bit8(mag_registry::QMC5883L_INFO, mag_registry::QMC5883L_INFO_DRDY_BIT)
    }

    /// Vérifie si le capteur n'est pas en saturation (utile pour le moteur !)
    fn is_overflow(&self) -> Result<bool, Box<dyn Error>> {
        self.i2c.lecture_bit8(mag_registry::QMC5883L_INFO, mag_registry::QMC5883L_INFO_OVL_BIT)
    }

    /// Vérifie si le mode "skip" n'est pas actif
    fn is_data_skip(&self) -> Result<bool, Box<dyn Error>> {
        self.i2c.lecture_bit8(mag_registry::QMC5883L_INFO, mag_registry::QMC5883L_INFO_DOR_BIT)
    }

    /// Défini le mode du capteur
    fn set_mode(&self, param: u8) -> Result<(), Box<dyn Error>> {
        self.i2c.ecriture_bits8(mag_registry::QMC5883L_SETTINGS, mag_registry::QMC5883L_SETTINGS_MODE_BIT, mag_registry::QMC5883L_SETTINGS_SIZE, param)
    }

    /// Défini le sample rate de sortie du capteur
    fn set_output_rate(&self, param: u8) -> Result<(), Box<dyn Error>> {
        self.i2c.ecriture_bits8(mag_registry::QMC5883L_SETTINGS, mag_registry::QMC5883L_SETTINGS_ODR_BIT, mag_registry::QMC5883L_SETTINGS_SIZE, param)
    }

    /// Défini le scale du capteur
    fn set_scale(&self, param: u8) -> Result<(), Box<dyn Error>> {
        self.i2c.ecriture_bits8(mag_registry::QMC5883L_SETTINGS, mag_registry::QMC5883L_SETTINGS_RNG_BIT, mag_registry::QMC5883L_SETTINGS_SIZE, param)
    }

    /// Défini le "Over Sample Rate"
    /// http://wiki.sunfounder.cc/images/7/72/QMC5883L-Datasheet-1.0.pdf (Page 17)
    fn set_osr(&self, param: u8) -> Result<(), Box<dyn Error>> {
        self.i2c.ecriture_bits8(mag_registry::QMC5883L_SETTINGS, mag_registry::QMC5883L_SETTINGS_OSR_BIT, mag_registry::QMC5883L_SETTINGS_SIZE, param)
    }

    /// Défini le Set/Reset
    fn set_setreset(&self, activate: bool) -> Result<(), Box<dyn Error>> {
        self.i2c.ecriture_bit8(mag_registry::QMC5883L_SETRESET, 0, activate)
    }
}

impl MagSensor for QMC5883L {
    /// Récupére la valeur "magnétique?" en X (RAW)
    fn get_mag_x_raw(&self) -> Result<i16, Box<dyn Error>> {
        let mag_x_h = self.i2c.lecture_word(mag_registry::QMC5883L_X_H)?;
        let mag_x_l = self.i2c.lecture_word(mag_registry::QMC5883L_X_L)?;
        Ok(((mag_x_h as i16) << 8) | mag_x_l as i16)
    }

    /// Récupére la valeur "magnétique?" en Y (RAW)
    fn get_mag_y_raw(&self) -> Result<i16, Box<dyn Error>> {
        let mag_y_h = self.i2c.lecture_word(mag_registry::QMC5883L_Y_H)?;
        let mag_y_l = self.i2c.lecture_word(mag_registry::QMC5883L_Y_L)?;
        Ok(((mag_y_h as i16) << 8) | mag_y_l as i16)
    }

    /// Récupére la valeur "magnétique?" en Z (RAW)
    fn get_mag_z_raw(&self) -> Result<i16, Box<dyn Error>> {
        let mag_z_h = self.i2c.lecture_word(mag_registry::QMC5883L_Z_H)?;
        let mag_z_l = self.i2c.lecture_word(mag_registry::QMC5883L_Z_L)?;
        Ok(((mag_z_h as i16) << 8) | mag_z_l as i16)
    }

    /// Permet de récupérer le status du capteur
    fn get_status(&self) -> u8 {
        self.status
    }
}