#![allow(unused)]

use std::{error::Error, task::Poll};
use std::fmt;
use tokio_stream::Stream;
use std::time::Duration;
use std::thread::sleep;
use std::time::Instant;
use nalgebra::Vector3;
use rppal::i2c::I2c;
use crate::i2c::I2CBit;

use crate::sensors::analog::registry;

pub struct Analog {
    i2c: I2c,
}

// Voir documentation : https://www.ti.com/lit/ds/symlink/ads1118.pdf

impl Analog {
    /// Constructeur
    pub fn new() -> anyhow::Result<Self> {
        println!("[ANALOG] Connexion I2C ...");

        // Prépare le I2C
        let i2c = I2c::new();
        match i2c {
            Ok(mut i2c) => {
                i2c.set_slave_address(registry::ANALOG_ADDR)?;

                // Créer l'objet et commence l'initialisation
                let mut analog = Analog {
                    i2c: i2c,
                };

                analog.init()?;

                Ok(analog)
            }

            Err(e) => {
                println!("[ANALOG] ERREUR: {}", e.to_string());
                Err(anyhow::anyhow!(e))
            }
        }
    }

    // Permet l'initialisation du module avec les valeurs demandées
    fn init(&mut self) -> anyhow::Result<()> {
        println!("[ANALOG] Initialisation ...");
        self.reset()?;
        self.set_datarate(registry::ADS1115_CONFIG_DR_128_VAL);
        self.set_mode(true);
        Ok(())
    }

    /// Réinitialise le module avec les valeurs par défaut
    fn reset(&self) -> anyhow::Result<()> {
        self.i2c.ecriture_dword(registry::ADS1115_CONFIG, 0x8583)?;
        self.set_lo_thresh(0x8000)?;
        self.set_hi_thresh(0x7FFF)?;
        Ok(())
    }

    /// Défini le seuil bas
    fn set_lo_thresh(&self, seuil: u16) -> anyhow::Result<()> {
        self.i2c.ecriture_dword(registry::ADS1115_LO_THRESH, seuil)
    }

    /// Défini le seuil haut
    fn set_hi_thresh(&self, seuil: u16) -> anyhow::Result<()> {
        self.i2c.ecriture_dword(registry::ADS1115_HI_THRESH, seuil)
    }
    
    /// Défini les inputs
    fn set_input(&self, input: u16) -> anyhow::Result<()> {
        self.i2c.ecriture_bits16(registry::ADS1115_CONFIG, registry::ADS1115_CONFIG_MUX_BIT, registry::ADS1115_CONFIG_MUX_LEN, input)
    }

    /// Active le mode Single-Shot ou le mode conversion continue (True => Single Shot)
    fn set_mode(&self, state: bool) -> anyhow::Result<()> {
        self.i2c.ecriture_bit16(registry::ADS1115_CONFIG, registry::ADS1115_CONFIG_MODE_BIT, state)
    }

    /// Défini le data rate
    fn set_datarate(&self, dr: u16) -> anyhow::Result<()> {
        self.i2c.ecriture_bits16(registry::ADS1115_CONFIG, registry::ADS1115_CONFIG_DR_BIT, registry::ADS1115_CONFIG_DR_LEN, dr)
    }

    /// Défini le gain
    fn set_gain(&self, gain: u16) -> anyhow::Result<f32>  {
        self.i2c.ecriture_bits16(registry::ADS1115_CONFIG, registry::ADS1115_CONFIG_PGA_BIT, registry::ADS1115_CONFIG_PGA_LEN, gain)?;
        match gain {
            registry::ADS1115_CONFIG_PGA_FSR_6_144_VAL => {
                return Ok((6.144*2.0)/2.0_f32.powf(16.0));
            },
            registry::ADS1115_CONFIG_PGA_FSR_4_096_VAL => {
                return Ok((4.096*2.0)/2.0_f32.powf(16.0));
            },
            registry::ADS1115_CONFIG_PGA_FSR_2_048_VAL => {
                return Ok((2.048*2.0)/2.0_f32.powf(16.0));
            },
            registry::ADS1115_CONFIG_PGA_FSR_1_024_VAL => {
                return Ok((1.024*2.0)/2.0_f32.powf(16.0));
            },
            registry::ADS1115_CONFIG_PGA_FSR_0_512_VAL => {
                return Ok((0.512*2.0)/2.0_f32.powf(16.0));
            },
            registry::ADS1115_CONFIG_PGA_FSR_0_256_1_VAL => {
                return Ok((0.256*2.0)/2.0_f32.powf(16.0));
            },
            registry::ADS1115_CONFIG_PGA_FSR_0_256_2_VAL => {
                return Ok((0.256*2.0)/2.0_f32.powf(16.0));
            },
            default => {
                println!("[ANALOG] Gain inconnu, défini à 1 par défaut.");
                return Ok(1.0);
            }
        }
    }

    /// Vérifie si une conversion est en cours
    fn is_conversion_progress(&self) -> anyhow::Result<bool> {
        let in_progress = self.i2c.lecture_bit16(registry::ADS1115_CONFIG, registry::ADS1115_CONFIG_OS_BIT)?;
        Ok(!in_progress)
    }

    /// Démarre une conversion (En Single Mode)
    fn start_conversion(&self) -> anyhow::Result<()> {
        self.i2c.ecriture_bit16(registry::ADS1115_CONFIG, registry::ADS1115_CONFIG_OS_BIT, true)
    }

    /// Lecture des données de tension (RAW)
    fn get_voltage_raw(&self) -> anyhow::Result<u16> {
        self.i2c.lecture_dword(registry::ADS1115_CONVERSION)
    }

    /// Lecture des données de tension
    fn get_voltage(&self, input: u16, gain: u16) -> anyhow::Result<f32> {
        // Défini les paramètres à utiliser
        self.set_input(input);
        let gain_adc = self.set_gain(gain)?;

        // Active un Sigle Shot
        self.start_conversion()?;

        // Attend que la valeur soit bien obtenable
        while self.is_conversion_progress()? {}

        let mut raw = self.get_voltage_raw()?;
        if raw > 65500 || raw < 100 {
            raw = 0;
        }

        // Retourne la valeur obtenue
        Ok((((raw as f32) * gain_adc) * registry::ANALOG_BATT_GAIN))
    }

    /// Récupére la valeur de la batterie
    pub fn get_battery(&mut self) -> anyhow::Result<f32> {
        self.get_voltage(registry::ADS1115_CONFIG_MUX_AIN0_AIN1_VAL, registry::ADS1115_CONFIG_PGA_FSR_4_096_VAL)
    }
}
