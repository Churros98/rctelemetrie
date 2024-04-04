#![allow(unused)]

use rppal::uart::Uart;
use rppal::uart::Parity;
use bincode::{config, Decode, Encode};
use std::path::Path;
use std::error::Error;
use std::fmt;

#[derive(Encode, Decode, Clone, Debug, Copy)]
pub struct GPSData {
    pub status: u8,
    pub lat: f32,
    pub dir_lat: u8,
    pub long: f32,
    pub dir_long: u8,
    pub decli_mag: f32,
    pub cap_vrai: f32,
    pub cap_mag: f32,
    pub vitesse_sol: f32,
}

impl fmt::Display for GPSData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LAT: {} LONG: {}", self.lat, self.long)
    }
}

#[derive(Debug)]
pub struct GPS {
    uart: Uart,
    message: Vec<u8>,
    status: u8,
    lat: f32,
    dir_lat: u8,
    long: f32,
    dir_long: u8,
    decli_mag: f32,
    cap_vrai: f32,
    cap_mag: f32,
    vitesse_sol: f32,
}

impl GPS {
    /// Constructeur
    pub fn new() -> Result<Self, Box<dyn Error>> {
        println!("[GPS] Initialisation ...");
        let uart_str = Path::new("/dev/serial0");
        let uart = Uart::with_path(uart_str, 38400, Parity::None, 8, 1);
        
        match uart {
            Ok(uart) => {
                println!("[GPS] Initialisé.");
        
                let gps = GPS {
                    uart: uart,
                    message: Vec::new(),
                    status: 0x0,
                    lat: 0.0,
                    dir_lat: b'N',
                    long: 0.0,
                    dir_long: b'W',
                    decli_mag: 0.0,
                    cap_vrai: 0.0,
                    cap_mag: 0.0,
                    vitesse_sol: 0.0,
                };
        
                Ok(gps)
            }
            Err(e) => {
                println!("[GPS] ERREUR: {}", e.to_string());
                Err("GPSINIT")?
            }
        }
    }

    // Traite les nouveaux messages (Trame entière)
    fn trame_nmea(&mut self, message: String) {
        let message_len = message.chars().count();
        let message_split: Vec<&str> = message.split(',').collect();

        if message_split.len() <= 0 {
            return;
        }

        match message_split[0] {
            "GPGLL" => {
                if message_split.len() >= 5 {
                    self.lat = message_split[1].parse::<f32>().unwrap_or(0.0);
                    self.long = message_split[3].parse::<f32>().unwrap_or(0.0);
                    
                    let dir_lat = message_split[2].as_bytes();
                    if dir_lat.len() > 0 {
                        self.dir_lat = message_split[2].as_bytes()[0];
                    }

                    let dir_long = message_split[4].as_bytes();
                    if dir_long.len() > 0 {
                        self.dir_long = message_split[4].as_bytes()[0];
                    }

                    //println!("[GPS] Réception coordonnées : ({} {}) ({} {})", lat, self.dir_lat as char, long, self.dir_long as char);
                }
            },
            "GPVTG" => {
                if message_split.len() >= 8 {
                    self.cap_vrai = message_split[1].parse::<f32>().unwrap_or(0.0);
                    self.cap_mag = message_split[3].parse::<f32>().unwrap_or(0.0);
                    self.vitesse_sol = message_split[7].parse::<f32>().unwrap_or(0.0);

                    //println!("[GPS] Réception cap et vitesse : (Cv: {}) (Cm: {}) (Vs: {})", cap_vrai, cap_mag, vitesse_sol);
                }
            },
            "GPRMC" => {
                if message_split.len() >= 12 {
                    self.decli_mag = message_split[11].parse::<f32>().unwrap_or(0.0);
                    //println!("[GPS] Déclinaison magnétique : {}", decli_mag);
                }
            },
            "GPGGA" => {
                if message_split.len() >= 9 {
                    if message_split[6] == "1" || message_split[6] == "2" {
                        self.status |= 0x2; // Fix OK.
                    } else {
                        self.status &= (0x2 ^ 0xFF); // Fix KO.
                    }
                }
            }
            &_ => {
                //println!("[GPS] Trame inconnu: {}", message_split[0])
            }
        }
    }

    // Permet la lecture des données de l'UART
    pub fn update(&mut self) -> Result<(), Box<dyn Error>> {
        let mut chars: &mut [u8; 255] = &mut [0;255];
        let size = self.uart.read(chars)?;

        // Pas de données ...
        if size <= 0 {
            return Ok(());
        }

        // Parcour l'ensemble des données reçus
        for index in 0..size {
            let char = chars[index];

            // Si '$', alors c'est que nous avons atteint un nouveau message. Traiter l'ancien.
            if char == ('$' as u8) {
                let message = self.message.clone();
                self.trame_nmea(String::from_utf8(message)?);
                self.message.clear();
                continue;
            }

            if char.is_ascii() && (char != b'\n' || char != b'\r'){
                self.message.push(char);
            }
        }

        Ok(())
    }

    /// Lecture des coordonnées actuel
    pub fn read_values(&self) -> GPSData {
        GPSData {
            status: self.status,
            lat: self.lat,
            dir_lat: self.dir_lat,
            long: self.long,
            dir_long: self.dir_long,
            decli_mag: self.decli_mag,
            cap_vrai: self.cap_vrai,
            cap_mag: self.cap_mag,
            vitesse_sol: self.vitesse_sol,
        }
    }


    /// Retourne des données vide
    pub fn empty() -> GPSData {
        GPSData {
            status: 0xFF,
            lat: 0.0,
            dir_lat: b'N',
            long: 0.0,
            dir_long: b'W',
            decli_mag: 0.0,
            cap_vrai: -1.0,
            cap_mag: -1.0,
            vitesse_sol: -1.0,
        } 
    }
}