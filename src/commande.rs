use bincode::config;
use bincode::error::DecodeError;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::watch::Sender;
use tokio::net::TcpStream;
use std::io::Error;
use std::time::Duration;
use tokio::time::sleep;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use std::time::SystemTime;

use crate::actuator::{self, ActuatorData};
use crate::Acurator;

pub struct Commande {
    is_stop: Arc<AtomicBool>,
    acurator_tx: Arc<Mutex<Sender<ActuatorData>>>,
    ip: String,
    port: i32,
}

/// Stream des données de contrôle (envoyé par le proxy)
impl Commande {
    pub fn new(ip: &String, port: i32, is_stop: Arc<AtomicBool>, acurator_tx: Arc<Mutex<Sender<ActuatorData>>>) -> Result<Self, Box<dyn std::error::Error>> {
        println!("[COMMANDE] Initialisation ...");

        Ok(Commande {
            is_stop: is_stop,
            acurator_tx: acurator_tx,
            ip: ip.clone(),
            port: port,
        })
    }

    /// Permet de gérer les erreurs de connexion.
    async fn error(&self, client: &mut TcpStream, e: Error) {
        let client_addr = client.local_addr().unwrap();

        println!("[COMMANDE][{}] ERREUR: {}.", client_addr.to_string(), e);
        if client.shutdown().await.is_err() {
            println!("[COMMANDE] Impossible de fermer la connexion.");
        }
    }

    pub async fn update(&self) {
        let config = config::standard();

        while !self.is_stop.load(Ordering::Relaxed) {
            println!("[COMMANDE] Tentative de connexion au serveur ...");
            let stream = TcpStream::connect(format!("{}:{}", self.ip, self.port)).await;
    
            if stream.is_err() {
                println!("[COMMANDE] Connexion au serveur impossible. Prochaine tentative dans 15 secondes ...");
                sleep(Duration::from_millis(15000)).await;
                continue;
            }
    
            let mut client = stream.unwrap();
    
            println!("[COMMANDE] Connecté au serveur.");
            
            let mut start_time = SystemTime::now();
            let mut fps = 0;

            // Je prépare un buffer avec des données vide à l'intérieur pour réception des instructions
            let mut actuator_buf: Vec<u8> = bincode::encode_to_vec(&Acurator::empty(), config).unwrap();

            // Gestion de la connexion actuel au serveur
            while !self.is_stop.load(Ordering::Relaxed)  {
                // Remplis le buffer
                if let Err(e) = client.read_exact(&mut actuator_buf).await {
                    self.error(&mut client, e).await; 
                    break;
                }

                // Je décode les instructions
                let decoder: Result<(ActuatorData, usize), DecodeError> = bincode::decode_from_slice(&actuator_buf[..], config);
                if decoder.is_err() {
                    println!("[COMMANDE] ERREUR: Impossible de décoder l'objet !");
                } else {
                    fps = fps + 1;
                    let (actuator_data, _len) = decoder.unwrap();
                    let _ = self.acurator_tx.lock().await.send(actuator_data);
                }
                
                if start_time.elapsed().unwrap().as_millis() > 1000 {
                    println!("[COMMANDE] DPS: {}", fps);
                    start_time = SystemTime::now();
                    fps = 0;
                }
            }

            println!("[COMMANDE] Déconnecté du serveur .. Tentative de reconnexion ...");
        }
    
        println!("[COMMANDE] Fin de réception des données de contrôle.");
    }
}