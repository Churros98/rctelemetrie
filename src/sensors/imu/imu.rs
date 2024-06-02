#![allow(unused)]

use std::{error::Error, task::Poll};
use std::fmt;
use tokio_stream::Stream;
use rppal::i2c::I2c;
use crate::i2c::I2CBit;
use std::time::Duration;
use std::thread::sleep;
use std::time::Instant;
use nalgebra::Vector3;
use crate::sensors::imu::registry;

pub struct IMU {
    i2c: I2c,
    gyro_cal: Vector3<f32>,
    accel_cal: Vector3<f32>,
    gyro_scale: f32,
    accel_scale: f32,
    angles: Vector3<f32>,
    temp: f32,
    last_measurment: Option<Instant>,
}

impl IMU {
    /// Constructeur
    pub fn new() -> anyhow::Result<Self> {
        println!("[IMU] Connexion I2C ...");

        // Prépare le I2C
        let i2c = I2c::new();
        match i2c {
            Ok(mut i2c) => {
                i2c.set_slave_address(registry::IMU_ADDR)?;

                // Créer l'objet et commence l'initialisation
                let mut imu = Self {
                    i2c: i2c,
                    gyro_cal: Vector3::new(0.0, 0.0, 0.0),
                    accel_cal: Vector3::new(0.0, 0.0, 0.0),
                    gyro_scale: 131.0,
                    accel_scale: 16384.0,
                    angles: Vector3::new(0.0, 0.0, 0.0),
                    temp: 0.0,
                    last_measurment: Option::None,
                };

                // Prépare le module
                imu.reset()?;
                imu.init_module()?;
                imu.calibration_imu()?;

                // Vérification
                imu.debug_get_info()?;

                Ok(imu)
            }

            Err(e) => {
                println!("[IMU] ERREUR: {}", e.to_string());
                Err(anyhow::anyhow!(e))
            }
        }
    }

    pub fn debug_get_info(&self) -> anyhow::Result<()>  {
        let clock = self.get_clock_source()?;
        let sleep = self.is_sleep_mode()?;
        let gyro_scale_range: u8 = self.get_fullscale_gyro_range()?;
        let accel_scale_range = self.get_fullscale_accel_range()?;
        let temp_enable = self.is_temp_sensor_enable()?;
        let who = self.whoami()?;
        let i2cbypass = self.get_i2c_bypass_enable()?;

        println!("[IMU] Who i am: {}", who);
        println!("[IMU] Temp. Enable: {}", temp_enable);
        println!("[IMU] I2C Bypass Enable: {}", i2cbypass);
        println!("[IMU] Sleep: {}", sleep);
        println!("[IMU] Clock source: {:#04x}", clock);
        println!("[IMU] Gyro scale range: {:#04x}", gyro_scale_range);
        println!("[IMU] Accel scale range: {:#04x}", accel_scale_range);
        Ok(())
    }

    /// Qui suis-je ?
    pub fn whoami(&self) -> anyhow::Result<u8>  {
        self.i2c.lecture_word(registry::MPU6050_RA_WHO_AM_I)
    }

    /// Initialise rapidement le module avec des valeurs pré-défini
    pub fn init_module(&mut self) -> anyhow::Result<()>  {
        println!("[IMU] Initialisation ...");
        self.set_clock_source(registry::MPU6050_CLOCK_PLL_XGYRO)?;
        self.set_i2c_bypass_enable(true)?;
        self.set_temp_sensor_enable(true)?;
        self.set_sleep_mode(false)?;
        self.set_fullscale_accel_range(registry::MPU6050_ACCEL_FS_2)?;
        self.set_fullscale_gyro_range(registry::MPU6050_GYRO_FS_250)?;
        Ok(())
    }

    /// Réinitialise le capteur via le trigger de tous les resets
    pub fn reset(&self) -> anyhow::Result<()>  {
        self.i2c.ecriture_word(registry::MPU6050_RA_USER_CTRL, 0x07)?;
        self.i2c.ecriture_word(registry::MPU6050_RA_SIGNAL_PATH_RESET, 0x07)?;
        self.i2c.ecriture_word(registry::MPU6050_RA_PWR_MGMT_1, 0x80)?;
        Ok(())
    }

    /// Vérifie si le module est en veille
    pub fn is_sleep_mode(&self) -> anyhow::Result<bool>  {
        self.i2c.lecture_bit8(registry::MPU6050_RA_PWR_MGMT_1, registry::MPU6050_PWR1_SLEEP_BIT)
    }

    /// Défini le mode veille du module
    pub fn set_sleep_mode(&self, enable: bool) -> anyhow::Result<()>  {
        self.i2c.ecriture_bit8(registry::MPU6050_RA_PWR_MGMT_1, registry::MPU6050_PWR1_SLEEP_BIT, enable)
    }

    /// Vérifie si le capteur de temperature est bien activé
    pub fn is_temp_sensor_enable(&self) -> anyhow::Result<bool>  {
        let is_temp = self.i2c.lecture_bit8(registry::MPU6050_RA_PWR_MGMT_1, registry::MPU6050_PWR1_TEMP_DIS_BIT)?;
        Ok(!is_temp)
    }

    /// Défini l'activation du capteur de temperature
    pub fn set_temp_sensor_enable(&self, enable: bool) -> anyhow::Result<()>  {
        self.i2c.ecriture_bit8(registry::MPU6050_RA_PWR_MGMT_1, registry::MPU6050_PWR1_TEMP_DIS_BIT, !enable)
    }

    /// Récupére la source de l'horloge
    pub fn get_clock_source(&self) -> anyhow::Result<u8>  {
        self.i2c.lecture_bits8(registry::MPU6050_RA_PWR_MGMT_1, registry::MPU6050_PWR1_CLKSEL_BIT, registry::MPU6050_PWR1_CLKSEL_LENGTH)
    }
 
    /// Défini la source de l'horloge
    pub fn set_clock_source(&self, source: u8) -> anyhow::Result<()>  {
        self.i2c.ecriture_bits8(registry::MPU6050_RA_PWR_MGMT_1, registry::MPU6050_PWR1_CLKSEL_BIT, registry::MPU6050_PWR1_CLKSEL_LENGTH, source)
    }

    /// Récupére le scale du gyroscope
    pub fn get_fullscale_gyro_range(&self) -> anyhow::Result<u8>  {
        self.i2c.lecture_bits8(registry::MPU6050_RA_GYRO_CONFIG, registry::MPU6050_GCONFIG_FS_SEL_BIT, registry::MPU6050_GCONFIG_FS_SEL_LENGTH)
    }

    /// Défini le mode "Bypass" pour l'I2C Aux.
    pub fn set_i2c_bypass_enable(&self, enable: bool) -> anyhow::Result<()>  {
        self.i2c.ecriture_bit8(registry::MPU6050_RA_INT_PIN_CFG, registry::MPU6050_INTCFG_I2C_BYPASS_EN_BIT, enable)
    }

    /// Récupére le mode "Bypass" pour l'I2C Aux.
    pub fn get_i2c_bypass_enable(&self) -> anyhow::Result<bool>  {
        self.i2c.lecture_bit8(registry::MPU6050_RA_INT_PIN_CFG, registry::MPU6050_INTCFG_I2C_BYPASS_EN_BIT)
    }
    
    /// Défini le scale du gyroscope
    pub fn set_fullscale_gyro_range(&mut self, range: u8) -> anyhow::Result<()>  {
        match range {
            registry::MPU6050_GYRO_FS_250  => self.gyro_scale=131.0,
            registry::MPU6050_GYRO_FS_500  => self.gyro_scale=65.5,
            registry::MPU6050_GYRO_FS_1000 => self.gyro_scale=32.8,
            registry::MPU6050_GYRO_FS_2000 => self.gyro_scale=16.4,
            _ => println!("Gyro range invalide."),
        }
        
        self.i2c.ecriture_bits8(registry::MPU6050_RA_GYRO_CONFIG, registry::MPU6050_GCONFIG_FS_SEL_BIT, registry::MPU6050_GCONFIG_FS_SEL_LENGTH, range)
    }

    /// Récupére le scale de l'accélérométre
    pub fn get_fullscale_accel_range(&self) -> anyhow::Result<u8>  {
        self.i2c.lecture_bits8(registry::MPU6050_RA_ACCEL_CONFIG, registry::MPU6050_ACONFIG_AFS_SEL_BIT, registry::MPU6050_ACONFIG_AFS_SEL_LENGTH)
    }
    
    /// Défini le scale de l'accélérométre
    pub fn set_fullscale_accel_range(&mut self, range: u8) -> anyhow::Result<()>  {
        match range {
            registry::MPU6050_ACCEL_FS_2 => self.accel_scale=16384.0,
            registry::MPU6050_ACCEL_FS_4 => self.accel_scale=8192.0,
            registry::MPU6050_ACCEL_FS_8 => self.accel_scale=4096.0,
            registry::MPU6050_ACCEL_FS_16=> self.accel_scale=2048.0,
            _ => println!("Accel range invalide."),
        }

        self.i2c.ecriture_bits8(registry::MPU6050_RA_ACCEL_CONFIG, registry::MPU6050_ACONFIG_AFS_SEL_BIT, registry::MPU6050_ACONFIG_AFS_SEL_LENGTH, range)
    }

    ///////////////////////////////////
    /// GESTION DES MESURES
    ///////////////////////////////////

    /// Calibration de l'IMU
    pub fn calibration_imu(&mut self) -> anyhow::Result<()>  {
        println!("[IMU] Calibration ...");

        // Récupére ~500 mesures et fait une moyenne
        let mut offset_gyro = Vector3::new(0.0 as f32, 0.0 as f32, 0.0 as f32);
        let mut offset_accel = Vector3::new(0.0 as f32, 0.0 as f32, 0.0 as f32);

        for n in 0..500 {
            let mesure_gyro = self.get_gyro_raw()?;
            let mesure_accel = self.get_accel_raw()?;

            offset_gyro += mesure_gyro;
            offset_accel += mesure_accel;

            sleep(Duration::from_millis(5))
        }

        self.gyro_cal = offset_gyro / 500.0;
        self.accel_cal = offset_accel / 500.0;

        println!("Calibration GYRO: (X: {} Y: {} Z: {})", self.gyro_cal.x, self.gyro_cal.y, self.gyro_cal.z);
        println!("Calibration ACCEL: (X: {} Y: {} Z: {})", self.accel_cal.x, self.accel_cal.y, self.accel_cal.z);
        Ok(())
    }

    /// Récupére la température en °C du capteur
    pub fn get_actual_temp(&self) -> anyhow::Result<f32>  {
        let temp_h = self.i2c.lecture_word(registry::MPU6050_RA_TEMP_OUT_H)?;
        let temp_l = self.i2c.lecture_word(registry::MPU6050_RA_TEMP_OUT_L)?;
        let temp = ((temp_h as i16) << 8) | temp_l as i16;

        Ok((temp as f32/340.0) + 36.53)
    }

    /// Récupére l'accélération en X (RAW)
    pub fn get_accel_x(&self) -> anyhow::Result<i16>  {
        let accel_x_h = self.i2c.lecture_word(registry::MPU6050_RA_ACCEL_XOUT_H)?;
        let accel_x_l = self.i2c.lecture_word(registry::MPU6050_RA_ACCEL_XOUT_L)?;
        Ok(((accel_x_h as i16) << 8) | accel_x_l as i16)
    }

    /// Récupére l'accélération en Y (RAW)
    pub fn get_accel_y(&self) -> anyhow::Result<i16>  {
        let accel_y_h = self.i2c.lecture_word(registry::MPU6050_RA_ACCEL_YOUT_H)?;
        let accel_y_l = self.i2c.lecture_word(registry::MPU6050_RA_ACCEL_YOUT_L)?;
        Ok(((accel_y_h as i16) << 8) | accel_y_l as i16)
    }

    /// Récupére l'accélération en Z (RAW)
    pub fn get_accel_z(&self) -> anyhow::Result<i16>  {
        let accel_z_h = self.i2c.lecture_word(registry::MPU6050_RA_ACCEL_ZOUT_H)?;
        let accel_z_l = self.i2c.lecture_word(registry::MPU6050_RA_ACCEL_ZOUT_L)?;
        Ok(((accel_z_h as i16) << 8) | accel_z_l as i16)
    }

    /// Récupére la vitesse angulaire en X (RAW)
    pub fn get_gyro_x(&self) -> anyhow::Result<i16>  {
        let gyro_x_h = self.i2c.lecture_word(registry::MPU6050_RA_GYRO_XOUT_H)?;
        let gyro_x_l = self.i2c.lecture_word(registry::MPU6050_RA_GYRO_XOUT_L)?;
        Ok(((gyro_x_h as i16) << 8) | gyro_x_l as i16)
    }

    /// Récupére la vitesse angulaire en Y (RAW)
    pub fn get_gyro_y(&self) -> anyhow::Result<i16>  {
        let gyro_y_h = self.i2c.lecture_word(registry::MPU6050_RA_GYRO_YOUT_H)?;
        let gyro_y_l = self.i2c.lecture_word(registry::MPU6050_RA_GYRO_YOUT_L)?;
        Ok(((gyro_y_h as i16) << 8) | gyro_y_l as i16)
    }

    /// Récupére la vitesse angulaire en Z (RAW)
    pub fn get_gyro_z(&self) -> anyhow::Result<i16>  {
        let gyro_z_h = self.i2c.lecture_word(registry::MPU6050_RA_GYRO_ZOUT_H)?;
        let gyro_z_l = self.i2c.lecture_word(registry::MPU6050_RA_GYRO_ZOUT_L)?;
        Ok(((gyro_z_h as i16) << 8) | gyro_z_l as i16)
    }

    /// Récupére l'accélération dans un vecteur (RAW)
    pub fn get_accel_raw(&self) -> anyhow::Result<Vector3<f32>>  {
        let accel_x = self.get_accel_x()? as f32;
        let accel_y = self.get_accel_y()? as f32;
        let accel_z = self.get_accel_z()? as f32;

        Ok(Vector3::new(accel_x, accel_y, accel_z))
    }

    /// Récupére la vitesse angulaire dans un vecteur (RAW)
    pub fn get_gyro_raw(&self) -> anyhow::Result<Vector3<f32>>   {
        let gyro_x = self.get_gyro_x()? as f32;
        let gyro_y = self.get_gyro_y()? as f32;
        let gyro_z: f32 = self.get_gyro_z()? as f32;

        Ok(Vector3::new(gyro_x, gyro_y, gyro_z))
    }

    /// Récupére l'accélération dans un vecteur
    pub fn get_accel(&self) -> anyhow::Result<Vector3<f32>>  {
        let mut accel_measurement = self.get_accel_raw()?;
        Ok(accel_measurement / self.accel_scale)
    }

    /// Récupére la vitesse angulaire dans un vecteur
    pub fn get_gyro(&self) -> anyhow::Result<Vector3<f32>>  {
        let mut gyro_measurement = self.get_gyro_raw()? - self.gyro_cal;
        Ok(gyro_measurement / self.gyro_scale)
    }

    /// Récupére un angle d'euler à partir d'un filtre complémentaire, du gyroscope et de l'accélération
    pub fn get_angles(&self) -> Vector3<f32> {
        self.angles
    }

    /// Récupére la température enregistrer depuis la dernière update
    pub fn get_temp(&self) -> f32 {
        self.temp
    }

    /// Lis et mets à jour les valeurs de l'IMU
    pub fn update(&mut self) -> anyhow::Result<()>  {
        let acceleration = self.get_accel()?;
        let gyroscope = self.get_gyro()?;

        // Récupére la température
        self.temp = self.get_actual_temp()?;
        
        // Défini le temps de la 1er mesure, évite un drift des le début
        if self.last_measurment.is_none() {
            self.last_measurment = Some(Instant::now());
        }

        // Récupére le temps écoulé depuis la dernière mesure en secondes
        let elapsed_time = self.last_measurment.unwrap().elapsed();

        // Remet le compteur à 0
        self.last_measurment = Some(Instant::now());

        // Calcul d'angle en degrée via l'accéléromètre (2D)
        let accel_pitch  = (acceleration.y.atan2(acceleration.z)).to_degrees(); // Pitch (SUR)
        let accel_roll = (acceleration.x.atan2(acceleration.z)).to_degrees(); // Roll (SUR)

        // Calcul d'angle en degrée via le gyroscope (3D)
        let gyroscope_pitch = self.angles.x + (gyroscope.x * elapsed_time.as_secs_f32()); // Pitch (SUR)
        let gyroscope_roll = self.angles.y - (gyroscope.y * elapsed_time.as_secs_f32()); // Roll (SUR)
        let gyroscope_yaw = self.angles.z + (gyroscope.z * elapsed_time.as_secs_f32());

        // Filte complémentaire
        self.angles.x = 0.98 * gyroscope_pitch + 0.02 * accel_pitch; // Pitch
        self.angles.y = 0.98 * gyroscope_roll + 0.02 * accel_roll; // Roll
        self.angles.z = gyroscope_yaw; // Très imprécis, utiliser le magnétomètre

        Ok(())
    }
}