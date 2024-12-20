mod actuators;
mod database;
mod sensors;

#[cfg(feature = "real-sensors")]
mod i2c;

#[cfg(feature = "real-sensors")]

use std::{
    sync::Arc,
    time::Duration,
};

use database::Database;
use futures::StreamExt;
use tokio::time::{sleep, timeout};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use zbus::{
    fdo,
    names::InterfaceName,
    Connection,
};

#[cfg(unix)]
use tokio::signal::unix::SignalKind;
use tokio::signal::{self};

const DEAD_TIMEOUT: u64 = 500;

#[tokio::main]
async fn main() {
    // Récupére le UUID de la voiture
    let uuid = std::env::var("UUID").unwrap();

    // Vérifie si le UUID est valide
    let uuid = match Uuid::parse_str(&uuid) {
        Ok(uuid) => uuid,
        Err(_) => {
            eprintln!("[MAIN] UUID invalide. Veuillez spécifier un UUID valide en variable d'environnement `UUID`.");
            return;
        }
    };

    let token = CancellationToken::new();

    // Préparation de la base de donnée
    println!("[DB] Connexion à la base de donnée ...");
    let db = match Database::new(uuid).await {
        Ok(db) => {
            println!("[DB] Connexion établie.");
            Arc::new(db)
        }
        Err(e) => {
            panic!("[DB] Erreur de connexion: {}", e);
        }
    };

    // Capteur
    {
        let token = token.child_token();

        let mut reader = sensors::reader::Reader::new(token.clone()).expect("[CAPTEURS] Impossible de gérer les capteurs.");
        let db = db.clone();
    
        tokio::spawn(async move {
            while !token.is_cancelled() {
                if let Some(data) = reader.next().await {
                    if let Ok(data) = data {
                        let _ = db.send_analog(data.analog).await;
                        let _ = db.send_nav(data.gps, data.mag, data.imu).await;
                    }

                    sleep(Duration::from_millis(1000 / 30)).await;
                }
            }
        });
    }

    // Modem 4G
    {
        let token = token.child_token();
        let db = db.clone();

        #[cfg(feature = "real-sensors")]
        {
            let connection = match Connection::system().await {
                Ok(conn) => conn,
                Err(e) => {
                    eprintln!("[MODEM] Impossible de gérer le D-BUS: {}", e);
                    return;
                }
            };

            tokio::spawn(async move {
                let proxy = match fdo::PropertiesProxy::builder(&connection)
                    .destination("org.freedesktop.ModemManager1")
                    .unwrap()
                    .path("/org/freedesktop/ModemManager1/Modem/0")
                    .unwrap()
                    .interface("org.freedesktop.DBus.Properties")
                    .unwrap()
                    .build()
                    .await {
                        Ok(p) => p,
                        Err(e) => {
                            eprintln!("[MODEM] Impossible de créer le proxy: {}", e);
                            return;
                        }
                    };

                let interface = match InterfaceName::try_from("org.freedesktop.ModemManager1.Modem") {
                    Ok(i) => i,
                    Err(e) => {
                        eprintln!("[MODEM] Interface invalide: {}", e);
                        return;
                    }
                };

                while !token.is_cancelled() {
                    match proxy.get(interface.clone(), "SignalQuality").await {
                        Ok(signal_quality) => {
                            if let Ok(signal) = <(u32, bool)>::try_from(signal_quality) {
                                println!("[MODEM] Signal: {}", signal.0);
                                if let Err(e) = db.send_modem(signal.0).await {
                                    eprintln!("[MODEM] Erreur d'envoi: {}", e);
                                }
                            }
                        }
                        Err(e) => eprintln!("[MODEM] Erreur de lecture: {}", e)
                    }

                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            });
        }

        #[cfg(feature = "fake-sensors")]
        {
            tokio::spawn(async move {
                let mut rng = rand::thread_rng();

                while !token.is_cancelled() {
                    let signal: u32 = rng.gen();
                    let _ = db.send_modem(signal).await;
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            });
        }
    }

    // Switch (Activation fonction unique)
    {
        let token = token.child_token();
        let db = db.clone();

        // Réinitialise les switchs
        if let Err(e) = db.reset_switchs().await {
            eprintln!("[SWITCH] Impossible de réinitialiser les switchs ({e})");
        }

        tokio::spawn(async move {
            #[cfg(feature = "real-actuators")]
            {
                let mut switch = match crate::actuators::switch::Switch::new() {
                    Ok(s) => s,
                    Err(e) => {
                        println!("[SWITCH] Erreur lors de l'init des switchs: {}", e);
                        return;
                    }
                };

                while !token.is_cancelled() {
                    match db.live_switch().await {
                        Ok(mut stream) => {
                            while !token.is_cancelled() {
                                if let Some(Ok(data)) = stream.next().await {
                                    if data.data.esc { switch.start_esc() } else { switch.stop_esc() };
                                }
                            }
                        }
                        Err(e) => eprintln!("[SWITCH] Erreur lors de la création du live: {}", e)
                    }
                }

                switch.stop_esc();
            }
        });
    }

    // Controle analogique
    {
        let token = token.child_token();
        let db = db.clone();
        tokio::spawn(async move {
            #[cfg(feature = "real-actuators")]
            {
                let mut motor = match crate::actuators::motor::Motor::new() {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("[CONTROL] Erreur lors de l'init moteur: {}", e);
                        return;
                    }
                };

                let mut steer = match crate::actuators::steering::Steering::new() {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("[CONTROL] Erreur lors de l'init steering: {}", e);
                        return;
                    }
                };

                while !token.is_cancelled() {
                    match db.live_control().await {
                        Ok(mut stream) => {
                            while !token.is_cancelled() {
                                match timeout(Duration::from_millis(DEAD_TIMEOUT), stream.next()).await {
                                    Ok(Some(Ok(data))) if data.action == surrealdb::Action::Update => {
                                        if let Err(e) = steer.set_steer(data.data.steer) {
                                            eprintln!("[CONTROL] Erreur lors du contrôle de la direction: {}", e)
                                        }

                                        if let Err(e) = motor.set_speed(data.data.speed) {
                                            eprintln!("[CONTROL] Erreur lors du contrôle moteur: {}", e)
                                        }
                                    }
                                    Ok(Some(Err(e))) => {
                                        eprintln!("[CONTROL] Erreur lors de l'update: {}", e);
                                    }
                                    Err(_) => {
                                        eprintln!("[CONTROL] Update tardif des données...");
                                        let _ = motor.set_speed(0.0);
                                    }
                                    _ => continue
                                }
                            }
                        }
                        Err(e) => eprintln!("[CONTROL] Erreur lors de la création du live: {}", e)
                    }
                }

                motor.safe_stop();
                steer.safe_stop();
            }

            #[cfg(feature = "fake-actuators")]
            {
                while !token.is_cancelled() {
                    match db.live_control().await {
                        Ok(mut stream) => {
                            while !token.is_cancelled() {
                                match timeout(Duration::from_millis(DEAD_TIMEOUT), stream.next()).await {
                                    Ok(Some(Ok(data))) if data.action == surrealdb::Action::Update => {
                                        println!(
                                            "[CONTROL] Steer: {} Speed: {}",
                                            data.data.steer, data.data.speed
                                        );
                                    }
                                    Ok(Some(Err(e))) => {
                                        eprintln!("[CONTROL] Erreur lors de l'update: {}", e);
                                    }
                                    Err(_) => {
                                        eprintln!("[CONTROL] Update tardif des données...");
                                    }
                                    _ => continue
                                }
                            }
                        }
                        Err(e) => eprintln!("[CONTROL] Erreur lors de la création du live: {}", e)
                    }
                }
            }
        });
    }

    #[cfg(unix)]
    {
        let mut test = tokio::signal::unix::signal(SignalKind::interrupt()).unwrap();
        tokio::select! {
            _ = test.recv() => {
                println!("Signal d'interruption reçu");
                token.cancel();
            },
            _ = signal::ctrl_c() => {
                println!("Signal de contrôle C reçu");
                token.cancel();
            },
        }
    }

    #[cfg(not(unix))]
    {
        tokio::select! {
            _ = signal::ctrl_c() => {
                println!("Signal de contrôle C reçu");
                token.cancel();
            },
        }
    }
}
