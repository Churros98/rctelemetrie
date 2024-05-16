use bincode::config;
use tokio::io::AsyncWriteExt;
use tokio::sync::watch::Receiver;
use tokio::net::TcpStream;
use std::io::Error;
use std::time::Duration;
use tokio::time::sleep;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use std::time::SystemTime;

use crate::sensors::SensorsData;

pub struct Telemetrie {
    is_stop: Arc<AtomicBool>,
    sensors_rx: Receiver<SensorsData>,
    ip: String,
    port: i32,
}

/// Stream des données de télémétrie (récupérer par les capteurs)
impl Telemetrie {
    pub fn new(ip: &String, port: i32, is_stop: Arc<AtomicBool>, sensors_rx: Receiver<SensorsData>) -> Result<Self, Box<dyn std::error::Error>> {
        println!("[TELEMETRIE] Initialisation ...");

        Ok(Telemetrie {
            is_stop: is_stop,
            sensors_rx: sensors_rx,
            ip: ip.clone(),
            port: port,
        })
    }

    /// Permet de gérer les erreurs de connexion.
    async fn error(&self, client: &mut TcpStream, e: Error) {
        let client_addr = client.local_addr().unwrap();

        println!("[TELEMETRIE][{}] ERREUR: {}.", client_addr.to_string(), e);
        if client.shutdown().await.is_err() {
            println!("[TELEMETRIE] Impossible de fermer la connexion.");
        }
    }

    pub async fn update(&self) {
        let config = config::standard();

        while !self.is_stop.load(Ordering::Relaxed) {
            println!("[TELEMETRIE] Tentative de connexion au serveur ...");
            let stream = TcpStream::connect(format!("{}:{}", self.ip, self.port)).await;
    
            if stream.is_err() {
                println!("[TELEMETRIE] Connexion au serveur impossible. Prochaine tentative dans 15 secondes ...");
                sleep(Duration::from_millis(15000)).await;
                continue;
            }
    
            let mut client = stream.unwrap();
    
            println!("[TELEMETRIE] Connecté au serveur.");
            
            let mut start_time = SystemTime::now();
            let mut fps = 0;

            // Gestion de la connexion actuel au serveur
            while !self.is_stop.load(Ordering::Relaxed)  {
                let sensors_data = self.sensors_rx.borrow().clone();
    
                // Prépare le buffer et écrit les données de télémétrie
                let sensors_buffer: Vec<u8> = bincode::encode_to_vec(&sensors_data, config).expect("Impossible d'encoder l'objet");

                match client.try_write(sensors_buffer.as_slice()) {
                    Ok(_n) => { fps = fps + 1; }
                    Err(e) => {self.error(&mut client, e).await; break;}
                }                
    
                if start_time.elapsed().unwrap().as_millis() > 1000 {
                    println!("[TELEMETRIE] DPS: {}", fps);
                    start_time = SystemTime::now();
                    fps = 0;
                }

                // J'attend entre les messages (~ 30 message par secondes)
                sleep(Duration::from_millis(33)).await;
            }
    
            println!("[TELEMETRIE] Déconnecté du serveur .. Tentative de reconnexion ...");
        }
    
        println!("[TELEMETRIE] Fin d'envoi des données de télémétrie.");
    }
}