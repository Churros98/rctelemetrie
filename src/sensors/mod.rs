#![allow(unused)]

use std::fmt;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::oneshot::Sender;
use tokio::sync::watch;
use self::analog::{Analog, AnalogData};
use self::imu::{IMUData, IMU};
use self::gps::{GPSData, GPS};
use self::mag::{MAGData, MAG};
use std::{error::Error, time::Duration};
use tokio::time::sleep;
use futures::join;
use tokio::sync::broadcast;
use bincode::{config, Decode, Encode};

pub mod imu;
pub mod gps;
pub mod mag;
pub mod analog;

#[derive(Clone, Encode, Decode, Debug, Copy)]
pub struct SensorsData {
    imu: IMUData,
    gps: GPSData,
    mag: MAGData,
    analog: AnalogData,
}

impl fmt::Display for SensorsData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IMU: {} GPS: {} MAG: {}", self.imu, self.gps, self.mag)
    }
}

pub struct Sensors {
    is_stop: Arc<AtomicBool>,
    tx: Arc<Mutex<watch::Sender<SensorsData>>>,
    gps: GPS,
    imu: IMU,
    mag: MAG,
    analog: Analog,
}

impl Sensors {
    pub fn empty() -> SensorsData {
        SensorsData {
            imu: IMU::empty(),
            gps: GPS::empty(),
            mag: MAG::empty(),
            analog: Analog::empty(),
        }
    }

    pub fn new(is_stop: Arc<AtomicBool>, tx: Arc<Mutex<watch::Sender<SensorsData>>>) -> Result<Self, Box<dyn Error>> {
        println!("[SENSORS] Initialisation ...");

        let gps = GPS::new()?;
        let imu = IMU::new()?;
        let mag = MAG::new()?;
        let analog = Analog::new()?;

        println!("[SENSORS] Capteurs initialisé !");

        Ok(Sensors {
            is_stop: is_stop,
            tx: tx,
            gps: gps,
            imu: imu,
            mag: mag,
            analog: analog,
        })
    }

    pub async fn update(&mut self) {
        let mut gps_alive = true;
        let mut imu_alive = true;
        let mut mag_alive = true;
        let mut analog_alive = true;

        println!("[SENSORS] Traitement des données ...");

        // Traite les données et les envois dans un format compatible
        while !self.is_stop.load(Ordering::Relaxed) {
            // Mise à jour et traitement des données
            if self.gps.update().is_err() {
                gps_alive = false;
            }
    
            if self.imu.update().is_err() {
                imu_alive = false;
            }
    
            if self.mag.update().is_err() {
                mag_alive = false;
            }

            if self.analog.update().is_err() {
                analog_alive = false;
            }

            // Récupération des données
            let mut gps_data = GPS::empty();
            let mut imu_data = IMU::empty();
            let mut mag_data = MAG::empty();
            let mut analog_data = Analog::empty();
    
            if gps_alive {
                gps_data = self.gps.read_values();
            }
    
            if imu_alive {
                imu_data = self.imu.read_values();
            }
    
            if mag_alive {
                mag_data = self.mag.read_values();
            }

            if analog_alive {
                analog_data = self.analog.read_values();
            }
    
            // Envoi des feedbacks au capteurs
            if mag_alive && gps_alive && gps_data.decli_mag != 0.0 {
                self.mag.feedback_set_mag_decl(gps_data.decli_mag);
            }
            
            // Préparation des données
            let sensors_data = SensorsData {
                imu: imu_data,
                gps: gps_data,
                mag: mag_data,
                analog: analog_data,
            };
    
            // Envoi des données au client
            {
                self.tx.lock().await.send(sensors_data);
            }
        }
    
        println!("[SENSORS] Arrêt de tous les capteurs ...");
    }
}
