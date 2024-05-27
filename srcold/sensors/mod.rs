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
use self::lte::LTE;
use self::mag::{MAGData, MAG};
use crate::sensors::lte::LTEData;
use std::{error::Error, time::Duration};
use tokio::time::sleep;
use futures::{join, StreamExt};
use tokio::sync::broadcast;
use bincode::{config, Decode, Encode};

pub mod imu;
pub mod gps;
pub mod mag;
pub mod lte;
pub mod analog;

#[derive(Clone, Encode, Decode, Debug, Copy)]
pub struct SensorsData {
    imu: IMUData,
    gps: GPSData,
    mag: MAGData,
    lte: LTEData,
    analog: AnalogData,
}

impl fmt::Display for SensorsData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "A: {} GPS: {} MAG: {} LTE: {} ANALOG: {}", self.imu, self.gps, self.mag, self.lte, self.analog)
    }
}

pub struct Sensors {
    // Général
    is_stop: Arc<AtomicBool>,
    tx: Arc<Mutex<watch::Sender<SensorsData>>>,
    
    // Capteurs
    gps: GPS,
    imu: IMU,
    mag: MAG,
    lte: LTE,
    analog: Analog,

    // Données
    data: SensorsData,
}

impl Sensors {
    /// Retourne des données de capteurs vide.
    pub fn empty() -> SensorsData {
        SensorsData {
            imu: IMU::empty(),
            gps: GPS::empty(),
            mag: MAG::empty(),
            lte: LTE::empty(),
            analog: Analog::empty(),
        }
    }

    /// Créer une instante de la gestion des capteuts
    pub async fn new(is_stop: Arc<AtomicBool>, tx: Arc<Mutex<watch::Sender<SensorsData>>>) -> Result<Self, Box<dyn Error>> {
        println!("[SENSORS] Initialisation ...");

        let gps = GPS::new()?;
        let imu = IMU::new()?;
        let mag = MAG::new()?;
        let lte = LTE::new()?;
        let analog = Analog::new()?;

        println!("[SENSORS] Capteurs initialisé !");

        Ok(Sensors {
            // Général
            is_stop: is_stop,
            tx: tx,
            
            // Capteurs
            gps: gps,
            imu: imu,
            mag: mag,
            lte: lte,
            analog: analog,

            // Données
            data: Sensors::empty(),
        })
    }

    /// Permet de récupérer les données des capteurs et de les envoyer à la télémétrie
    pub async fn update(&mut self) {
        println!("[SENSORS] Gestion des capteurs en cours ...");

        while !self.is_stop.load(Ordering::Relaxed) {
            // Analogique
            if self.analog.update().is_err() {
                self.data.analog = Analog::empty();
            } else {
                self.data.analog = self.analog.to_data();
            }

            // GPS
            if self.gps.update().is_err() {
                self.data.gps = GPS::empty();
            } else {
                self.data.gps = self.gps.to_data();
            }

            // IMU
            if self.imu.update().is_err() {
                self.data.imu = IMU::empty();
            } else {
                self.data.imu = self.imu.to_data();
            }
        
            // LTE
            if self.lte.update().is_err() {
                self.data.lte = LTE::empty();
            } else {
                self.data.lte = self.lte.to_data();
            }
        
            // Compas
            if self.mag.update().is_err() {
                self.data.mag = MAG::empty();
            } else {
                self.data.mag = self.mag.to_data();
            }

            // Envoi les données à la télémétrie
            self.tx.lock().await.send(self.data);
        }

        println!("[SENSORS] Arrêt de la gestion de tous les capteurs ...");
    }
}
