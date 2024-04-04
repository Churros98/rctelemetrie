use bincode::config;
use bincode::error::DecodeError;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::watch::{Receiver, Sender};
use tokio::net::TcpStream;
use std::io::Error;
use std::time::Duration;
use tokio::time::sleep;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use std::time::SystemTime;

use crate::sensors::SensorsData;
use crate::actuator::ActuatorData;
use crate::Acurator;

pub struct Telemetrie {
    is_stop: Arc<AtomicBool>,
    sensors_rx: Receiver<SensorsData>,
    acurator_tx: Arc<Mutex<Sender<ActuatorData>>>,
}

/// Stream des données de télémétrie (récupérer par les capteurs)
impl Telemetrie {
    pub fn new(is_stop: Arc<AtomicBool>, sensors_rx: Receiver<SensorsData>, acurator_tx: Arc<Mutex<Sender<ActuatorData>>>) -> Result<Self, Box<dyn std::error::Error>> {
        println!("[TELEMETRIE] Initialisation ...");

        Ok(Telemetrie {
            is_stop: is_stop,
            sensors_rx: sensors_rx,
            acurator_tx: acurator_tx,
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
            let stream = TcpStream::connect("192.168.1.102:1111").await;
    
            if stream.is_err() {
                println!("[TELEMETRIE] Connexion au serveur impossible. Prochaine tentative dans 15 secondes ...");
                sleep(Duration::from_millis(15000)).await;
                continue;
            }
    
            let mut client = stream.unwrap();
    
            println!("[TELEMETRIE] Connecté au serveur.");
            
            let mut start_time = SystemTime::now();
            let mut fps = 0;

            // Je prépare un buffer avec des données vide à l'intérieur pour réception des instructioons
            let mut actuator_buf: Vec<u8> = bincode::encode_to_vec(&Acurator::empty(), config).unwrap();

            // Gestion de la connexion actuel au serveur
            while !self.is_stop.load(Ordering::Relaxed)  {
                let sensors_data = *self.sensors_rx.borrow();
    
                // Prépare le buffer et écrit les données de télémétrie
                let sensors_buffer: Vec<u8> = bincode::encode_to_vec(&sensors_data, config).unwrap();
                match client.try_write(sensors_buffer.as_slice()) {
                    Ok(_n) => {}
                    Err(e) => {self.error(&mut client, e).await; break;}
                }

                // Reçois en retour des instructions
                match client.read_exact(&mut actuator_buf).await {
                    Ok(_n) => {}
                    Err(e) => {self.error(&mut client, e).await; break;}
                }

                // Je décode les instructions
                let decoder: Result<(ActuatorData, usize), DecodeError> = bincode::decode_from_slice(&actuator_buf[..], config);
                if decoder.is_err() {
                    println!("[TELEMETRIE] ERREUR: Impossible de décoder l'objet !");
                } else {
                    let (actuator_data, _len) = decoder.unwrap();
                    let _ = self.acurator_tx.lock().await.send(actuator_data);
                }
                
                fps = fps + 1;
    
                if start_time.elapsed().unwrap().as_millis() > 1000 {
                    println!("[TELEMETRIE] DPS: {}", fps);
                    start_time = SystemTime::now();
                    fps = 0;
                }

                // J'attend entre les messages (~ 50 message par secondes)
                sleep(Duration::from_millis(20)).await;
            }
    
            println!("[TELEMETRIE] Déconnecté du serveur .. Tentative de reconnexion ...");
        }
    
        println!("[TELEMETRIE] Fin d'envoi des données de télémétrie.");
    }
}