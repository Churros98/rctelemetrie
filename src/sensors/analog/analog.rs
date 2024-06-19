#![allow(unused)]

use crate::i2c::I2CBit;
use nalgebra::Vector3;
use rppal::i2c::I2c;
use std::fmt;
use std::thread::sleep;
use std::time::Duration;
use std::time::Instant;
use std::{error::Error, task::Poll};
use tokio_stream::Stream;

use crate::sensors::analog::registry;

pub struct Analog {}

// Voir documentation : https://www.ti.com/lit/ds/symlink/ads1118.pdf

impl Analog {
    /// Constructeur
    pub fn new(i2c: &mut I2c) -> anyhow::Result<Self> {        
        // Créer l'objet et commence l'initialisation
        let mut analog = Analog {};

        analog.set_slave(i2c)?;
        analog.init(i2c)?;

        Ok(analog)
    }

    fn set_slave(&self, i2c: &mut I2c) -> anyhow::Result<()> {
        i2c.set_slave_address(registry::ANALOG_ADDR)?;
        Ok(())
    }

    // Permet l'initialisation du module avec les valeurs demandées
    fn init(&mut self, i2c: &mut I2c) -> anyhow::Result<()> {
        println!("[ANALOG] Initialisation ...");
        self.reset(i2c)?;
        self.set_datarate(i2c, registry::ADS1115_CONFIG_DR_128_VAL);
        self.set_mode(i2c, true);
        Ok(())
    }

    /// Réinitialise le module avec les valeurs par défaut
    fn reset(&self, i2c: &mut I2c) -> anyhow::Result<()> {
        i2c.ecriture_dword(registry::ADS1115_CONFIG, 0x8583)?;
        self.set_lo_thresh(i2c, 0x8000)?;
        self.set_hi_thresh(i2c, 0x7FFF)?;
        Ok(())
    }

    /// Défini le seuil bas
    fn set_lo_thresh(&self, i2c: &mut I2c, seuil: u16) -> anyhow::Result<()> {
        i2c.ecriture_dword(registry::ADS1115_LO_THRESH, seuil)
    }

    /// Défini le seuil haut
    fn set_hi_thresh(&self, i2c: &mut I2c, seuil: u16) -> anyhow::Result<()> {
        i2c.ecriture_dword(registry::ADS1115_HI_THRESH, seuil)
    }

    /// Défini les inputs
    fn set_input(&self, i2c: &mut I2c, input: u16) -> anyhow::Result<()> {
        i2c.ecriture_bits16(
            registry::ADS1115_CONFIG,
            registry::ADS1115_CONFIG_MUX_BIT,
            registry::ADS1115_CONFIG_MUX_LEN,
            input,
        )
    }

    /// Active le mode Single-Shot ou le mode conversion continue (True => Single Shot)
    fn set_mode(&self, i2c: &mut I2c, state: bool) -> anyhow::Result<()> {
        i2c.ecriture_bit16(
            registry::ADS1115_CONFIG,
            registry::ADS1115_CONFIG_MODE_BIT,
            state,
        )
    }

    /// Défini le data rate
    fn set_datarate(&self, i2c: &mut I2c, dr: u16) -> anyhow::Result<()> {
        i2c.ecriture_bits16(
            registry::ADS1115_CONFIG,
            registry::ADS1115_CONFIG_DR_BIT,
            registry::ADS1115_CONFIG_DR_LEN,
            dr,
        )
    }

    /// Défini le gain
    fn set_gain(&self, i2c: &mut I2c, gain: u16) -> anyhow::Result<f32> {
        i2c.ecriture_bits16(
            registry::ADS1115_CONFIG,
            registry::ADS1115_CONFIG_PGA_BIT,
            registry::ADS1115_CONFIG_PGA_LEN,
            gain,
        )?;
        match gain {
            registry::ADS1115_CONFIG_PGA_FSR_6_144_VAL => {
                return Ok((6.144 * 2.0) / 2.0_f32.powf(16.0));
            }
            registry::ADS1115_CONFIG_PGA_FSR_4_096_VAL => {
                return Ok((4.096 * 2.0) / 2.0_f32.powf(16.0));
            }
            registry::ADS1115_CONFIG_PGA_FSR_2_048_VAL => {
                return Ok((2.048 * 2.0) / 2.0_f32.powf(16.0));
            }
            registry::ADS1115_CONFIG_PGA_FSR_1_024_VAL => {
                return Ok((1.024 * 2.0) / 2.0_f32.powf(16.0));
            }
            registry::ADS1115_CONFIG_PGA_FSR_0_512_VAL => {
                return Ok((0.512 * 2.0) / 2.0_f32.powf(16.0));
            }
            registry::ADS1115_CONFIG_PGA_FSR_0_256_1_VAL => {
                return Ok((0.256 * 2.0) / 2.0_f32.powf(16.0));
            }
            registry::ADS1115_CONFIG_PGA_FSR_0_256_2_VAL => {
                return Ok((0.256 * 2.0) / 2.0_f32.powf(16.0));
            }
            default => {
                println!("[ANALOG] Gain inconnu, défini à 1 par défaut.");
                return Ok(1.0);
            }
        }
    }

    /// Vérifie si une conversion est en cours
    fn is_conversion_progress(&self, i2c: &mut I2c) -> anyhow::Result<bool> {
        let in_progress =
            i2c.lecture_bit16(registry::ADS1115_CONFIG, registry::ADS1115_CONFIG_OS_BIT)?;
        Ok(!in_progress)
    }

    /// Démarre une conversion (En Single Mode)
    fn start_conversion(&self, i2c: &mut I2c) -> anyhow::Result<()> {
        i2c.ecriture_bit16(
            registry::ADS1115_CONFIG,
            registry::ADS1115_CONFIG_OS_BIT,
            true,
        )
    }

    /// Lecture des données de tension (RAW)
    fn get_voltage_raw(&self, i2c: &mut I2c) -> anyhow::Result<u16> {
        i2c.lecture_dword(registry::ADS1115_CONVERSION)
    }

    /// Lecture des données de tension
    fn get_voltage(&self, i2c: &mut I2c, input: u16, gain: u16) -> anyhow::Result<f32> {
        // Défini les paramètres à utiliser
        self.set_input(i2c, input);
        let gain_adc = self.set_gain(i2c, gain)?;

        // Active un Sigle Shot
        self.start_conversion(i2c)?;

        // Attend que la valeur soit bien obtenable
        while self.is_conversion_progress(i2c)? {}

        let mut raw = self.get_voltage_raw(i2c)?;
        if raw > 65500 || raw < 100 {
            raw = 0;
        }

        // Retourne la valeur obtenue
        Ok((((raw as f32) * gain_adc) * registry::ANALOG_BATT_GAIN))
    }

    /// Récupére la valeur de la batterie
    pub fn get_battery(&mut self, i2c: &mut I2c) -> anyhow::Result<f32> {
        self.set_slave(i2c)?;
        self.get_voltage(
            i2c,
            registry::ADS1115_CONFIG_MUX_AIN0_AIN1_VAL,
            registry::ADS1115_CONFIG_PGA_FSR_4_096_VAL,
        )
    }
}
