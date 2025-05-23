mod cli;
mod actuators;
mod database;
mod sensors;
mod config;

#[cfg(feature = "real-sensors")]
mod i2c;

use std::{
    sync::Arc,
    time::Duration,
};

use actuators::esc;
use clap::Parser;
use database::Database;
use futures::StreamExt;
use sensors::reader::SensorsData;
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
    // CLI
    let args = crate::cli::Cli::parse();

    // Vérifie si le UUID est valide
    if let Err(_) = Uuid::parse_str(&args.uuid) {
        eprintln!("[MAIN] UUID invalide. Veuillez spécifier un UUID valide.");
        return;
    }

    // Créer un canal pour le transfert des données des capteurs vers les actuateurs
    let (tx, mut rx) = tokio::sync::mpsc::channel::<SensorsData>(100);
    let token = CancellationToken::new();

    // Préparation de la base de donnée
    println!("[DB] Connexion à la base de donnée ...");
    let db = match Database::new(args.clone()).await {
        Ok(db) => {
            println!("[DB] Connexion établie.");
            Arc::new(db)
        }
        Err(e) => {
            panic!("[DB] Erreur de connexion: {}", e);
        }
    };

    // Récupére la configuration de la voiture
    let config = db.get_config().await.expect("[DB] Erreur lors de la récupération de la configuration.");

    // Capteur
    let sensors_task = {
        let token = token.child_token();
        let config = config.clone();

        let mut reader = sensors::reader::Reader::new(token.clone(), config).expect("[CAPTEURS] Impossible de gérer les capteurs.");
        let db = db.clone();
    
        tokio::spawn(async move {
            while !token.is_cancelled() {
                if let Some(data) = reader.next().await {
                    if let Ok(data) = data {
                        if tx.send(data.clone()).await.is_err() {
                            eprintln!("[CAPTEURS] Erreur lors de l'envoi des données.");
                        }
                        let _ = db.send_sensors(data).await;
                    }

                    sleep(Duration::from_millis(1000 / 30)).await;
                }
            }

            println!("[CAPTEURS] Fin de la tâche de mise à jour de la BDD.");
        })
    };

    // Modem 4G
    let modem_task = {
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

                            tokio::time::sleep(Duration::from_millis(500)).await;
                        }
                        Err(e) => {
                            eprintln!("[MODEM] Erreur de lecture: {}", e);
                            eprintln!("[MODEM] Arrêt du monitoring.");
                            break;
                        }
                    }
                }
            })
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
            })
        }
    };

    // Controle des actuateurs
    let actuators_task = {
        let token = token.child_token();
        let db = db.clone();
        let config = config.clone();

        tokio::spawn(async move {
            #[cfg(feature = "real-actuators")]
            {
                let mut sensors = rx.recv().await.unwrap();
                let mut motor = match crate::actuators::motor::Motor::new(config) {
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

                    // Live des commandes
                    match db.live_control().await {
                        Ok(mut stream) => {
                            let mut is_waiting = false;
                            while !token.is_cancelled() {
                                // Récupération des données des capteurs
                                match rx.try_recv() {
                                    Ok(data) => {
                                        sensors = data;
                                    }
                                    Err(_) => {
                                        eprintln!("[CONTROL] Erreur lors de la réception des données des capteurs.");
                                    }
                                }

                                // Vérifie si les commandes ont été mises à jour dans un laps de temps précis
                                match timeout(Duration::from_millis(DEAD_TIMEOUT), stream.next()).await {
                                    Ok(Some(Ok(data))) if data.action == surrealdb::Action::Update => {
                                        if let Err(e) = steer.set_steer(data.data.steer) {
                                            eprintln!("[CONTROL] Erreur lors du contrôle de la direction: {}", e)
                                        }

                                        if let Err(e) = motor.set_speed(data.data.speed, sensors.hall.speed) {
                                            eprintln!("[CONTROL] Erreur lors du contrôle moteur: {}", e)
                                        }

                                        is_waiting = false;
                                    }
                                    Ok(Some(Err(e))) => {
                                        eprintln!("[CONTROL] Erreur lors de l'update: {}", e);
                                    }
                                    Err(_) => {
                                        if !is_waiting {
                                            eprintln!("[CONTROL] Update tardif des données. Arrêt préventif du moteur.");
                                            let _ = motor.set_speed(0.0, sensors.hall.speed);
                                            is_waiting = true;
                                        }
                                    }
                                    _ => continue
                                }
                            }
                        }
                        Err(e) => eprintln!("[CONTROL] Erreur lors de la création du live: {}", e)
                    }
                }

                // Arrêt des actuateurs
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

            println!("[CONTROL] Fin de la tâche.")
        })
    };

    // Switch (Activation fonction unique)
    let _ = {
        let parent = token.clone();
        let token = token.child_token();
        let db = db.clone();

        // Réinitialise les switchs
        if let Err(e) = db.reset_switchs().await {
            eprintln!("[SWITCH] Impossible de réinitialiser les switchs ({e})");
        }

        tokio::spawn(async move {
            #[cfg(feature = "real-actuators")]
            {
                let mut esc = match crate::actuators::esc::ESC::new() {
                    Ok(esc) => esc,
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
                                    if data.data.esc { esc.start() } else { esc.stop() };
                                    if data.data.reload {
                                        println!("[SWITCH] Redémarrage du logiciel de télémétrie ...");
                                        parent.cancel();
                                        let _ = sleep(Duration::from_secs(2)).await;
                                        let _ = db.reset_switchs().await;
                                        panic!("[SWITCH] Arrêt de l'application forcé après 2 secondes. Reload.");
                                    }
                                }
                            }
                        }
                        Err(e) => eprintln!("[SWITCH] Erreur lors de la création du live: {}", e)
                    }
                }

                esc.stop();
            }

            println!("[SWITCH] Fin de la tâche.");
        })
    };

    // Signal (Signalisation du process)
    {
        let parent = token.clone();
        tokio::spawn(async move {
            #[cfg(unix)]
            {
                let mut test = tokio::signal::unix::signal(SignalKind::interrupt()).unwrap();
                tokio::select! {
                    _ = test.recv() => {
                        println!("Signal d'interruption reçu");
                        parent.cancel();
                    },
                    _ = signal::ctrl_c() => {
                        println!("Signal de contrôle C reçu");
                        parent.cancel();
                    },
                }
            }
        
            #[cfg(not(unix))]
            {
                tokio::select! {
                    _ = signal::ctrl_c() => {
                        println!("[MAIN] Signal de contrôle C reçu");
                        parent.cancel();
                    },
                }
            }
        });
    };

    println!("[MAIN] Attente de la fin d'exécution des tâches importante ...");
    while !(sensors_task.is_finished() && modem_task.is_finished() && actuators_task.is_finished()) {
        sleep(Duration::from_secs(1)).await
    }

    println!("[MAIN] Fin d'execution.");
}
