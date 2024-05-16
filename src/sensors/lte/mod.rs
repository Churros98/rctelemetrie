#![allow(unused)]

use zbus::blocking::Connection;
use bincode::{config, Decode, Encode};
use tokio_stream::Stream;
use zbus::message::Body;
use zbus::proxy;
use std::path::Path;
use std::error::Error;
use std::fmt;
use std::task::Poll;

#[derive(Encode, Decode, Clone, Debug, Copy)]
pub struct LTEData {
    pub status: u8,
    pub power: u32,
}

impl fmt::Display for LTEData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Power: {}", self.power)
    }
}

#[derive(Debug)]
pub struct LTE {
    connection: Connection,
    status: u8,
    power: u32,
}

#[proxy(
    interface = "org.freedesktop.ModemManager1",
    default_service = "org.freedesktop.ModemManager1.Modem",
    default_path = "/org/freedesktop/ModemManager/Modems/1"
)]
trait ModemManager {
    #[zbus(property)]
    fn SignalQuality(&self) -> zbus::Result<u8>;
}

impl LTE {
    /// Constructeur
    pub fn new() -> Result<Self, Box<dyn Error>> {
        println!("[LTE] Initialisation ...");

        let connection = Connection::system()?;
        
        let lte = LTE {
            connection: connection,
            status: 0x0,
            power: 999,
        };

        Ok(lte)
    }

    // Permet la mise à jour des données
    pub fn update(&mut self) -> Result<(), Box<dyn Error>> {
        self.power = 999;
        Ok(())
    }

    /// Retourne les données actuel
    pub fn to_data(&self) -> LTEData {
        LTEData {
            status: self.status,
            power: self.power,
        }
    }

    /// Retourne des données vide
    pub fn empty() -> LTEData {
        LTEData {
            status: 0xFF,
            power: 999,
        } 
    }
}
