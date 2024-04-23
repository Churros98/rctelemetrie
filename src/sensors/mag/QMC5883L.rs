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

pub struct QMC5883L {
    i2c: I2c,
    status: u8,
    mag_decl: f32,
    hard_cal: Vector3<f32>,
    soft_cal: Matrix3<f32>,
}

impl QMC5883L {
    /// Constructeur
    pub fn new() -> Result<Self, Box<dyn Error>> {
        println!("[QMC5883L] Initialisation ...");

        // Prépare le I2C
        let i2c = I2c::new();
        match i2c {
            Ok(mut i2c) => {
                i2c.set_slave_address(mag_registry::MAG_ADDR)?;

                // Créer l'objet et commence l'initialisation
                let mut mag = Self {
                    i2c: i2c,
                    mag_decl: 2.44,
                    hard_cal: Vector3::new(0.0, 0.0, 0.0),
                    soft_cal: Matrix3::new(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0),
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
    pub fn init_module(&mut self) -> Result<(), Box<dyn Error>> {
        println!("[MAG] Initialisation ...");
        self.set_setreset(true)?;
        self.set_mode(1)?;
        self.set_output_rate(0)?;
        self.set_scale(0)?;
        self.set_osr(0)?;
        self.status |= 0x1;
        Ok(())
    }
    
    /// Récupére la valeur "magnétique?" en X (RAW)
    pub fn get_mag_x_raw(&self) -> Result<i16, Box<dyn Error>> {
        let mag_x_h = self.i2c.lecture_word(mag_registry::QMC5883L_X_H)?;
        let mag_x_l = self.i2c.lecture_word(mag_registry::QMC5883L_X_L)?;
        Ok(((mag_x_h as i16) << 8) | mag_x_l as i16)
    }

    /// Récupére la valeur "magnétique?" en Y
    pub fn get_mag_y_raw(&self) -> Result<i16, Box<dyn Error>> {
        let mag_y_h = self.i2c.lecture_word(mag_registry::QMC5883L_Y_H)?;
        let mag_y_l = self.i2c.lecture_word(mag_registry::QMC5883L_Y_L)?;
        Ok(((mag_y_h as i16) << 8) | mag_y_l as i16)
    }

    /// Récupére la valeur "magnétique?" en Z
    pub fn get_mag_z_raw(&self) -> Result<i16, Box<dyn Error>> {
        let mag_z_h = self.i2c.lecture_word(mag_registry::QMC5883L_Z_H)?;
        let mag_z_l = self.i2c.lecture_word(mag_registry::QMC5883L_Z_L)?;
        Ok(((mag_z_h as i16) << 8) | mag_z_l as i16)
    }

    /// Récupére le "Chip ID"
    pub fn get_chip_id(&self) -> Result<u8, Box<dyn Error>> {
        self.i2c.lecture_word(mag_registry::QMC5883L_CHIP_ID)
    }

    /// Vérifie si les données sont disponible
    pub fn is_data_ready(&self) -> Result<bool, Box<dyn Error>> {
        self.i2c.lecture_bit8(mag_registry::QMC5883L_INFO, mag_registry::QMC5883L_INFO_DRDY_BIT)
    }

    /// Vérifie si le capteur n'est pas en saturation (utile pour le moteur !)
    pub fn is_overflow(&self) -> Result<bool, Box<dyn Error>> {
        self.i2c.lecture_bit8(mag_registry::QMC5883L_INFO, mag_registry::QMC5883L_INFO_OVL_BIT)
    }

    /// Vérifie si le mode "skip" n'est pas actif
    pub fn is_data_skip(&self) -> Result<bool, Box<dyn Error>> {
        self.i2c.lecture_bit8(mag_registry::QMC5883L_INFO, mag_registry::QMC5883L_INFO_DOR_BIT)
    }

    /// Défini le mode du capteur
    pub fn set_mode(&self, param: u8) -> Result<(), Box<dyn Error>> {
        self.i2c.ecriture_bits8(mag_registry::QMC5883L_SETTINGS, mag_registry::QMC5883L_SETTINGS_MODE_BIT, mag_registry::QMC5883L_SETTINGS_SIZE, param)
    }

    /// Défini le sample rate de sortie du capteur
    pub fn set_output_rate(&self, param: u8) -> Result<(), Box<dyn Error>> {
        self.i2c.ecriture_bits8(mag_registry::QMC5883L_SETTINGS, mag_registry::QMC5883L_SETTINGS_ODR_BIT, mag_registry::QMC5883L_SETTINGS_SIZE, param)
    }

    /// Défini le scale du capteur
    pub fn set_scale(&self, param: u8) -> Result<(), Box<dyn Error>> {
        self.i2c.ecriture_bits8(mag_registry::QMC5883L_SETTINGS, mag_registry::QMC5883L_SETTINGS_RNG_BIT, mag_registry::QMC5883L_SETTINGS_SIZE, param)
    }

    /// Défini le "Over Sample Rate"
    /// http://wiki.sunfounder.cc/images/7/72/QMC5883L-Datasheet-1.0.pdf (Page 17)
    pub fn set_osr(&self, param: u8) -> Result<(), Box<dyn Error>> {
        self.i2c.ecriture_bits8(mag_registry::QMC5883L_SETTINGS, mag_registry::QMC5883L_SETTINGS_OSR_BIT, mag_registry::QMC5883L_SETTINGS_SIZE, param)
    }

    /// Défini le Set/Reset
    pub fn set_setreset(&self, activate: bool) -> Result<(), Box<dyn Error>> {
        self.i2c.ecriture_bit8(mag_registry::QMC5883L_SETRESET, 0, activate)
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
        let corrected_mag = hard_mag; //* self.soft_cal;

        // Calcul du heading, prend en compte la déclinaison magnétique
        let mut heading = (corrected_mag.x.atan2(corrected_mag.y) * (180.0 / PI)) + self.mag_decl;
        heading = heading % 360.0;
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

