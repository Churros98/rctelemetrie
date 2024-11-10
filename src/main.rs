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
use zbus::{
    fdo,
    names::InterfaceName,
    zvariant,
    Connection,
};
use zvariant::OwnedValue;

#[cfg(unix)]
use tokio::signal::unix::SignalKind;
use tokio::signal::{self};

const DEAD_TIMEOUT: u64 = 500;

#[tokio::main]
async fn main() {
    let token = CancellationToken::new();

    // Préparation de la base de donnée
    println!("[DB] Connexion à la base de donnée ...");
    let db = match Database::new().await {
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
            let connection = Connection::system()
                .await
                .expect("Impossible de gérer le D-BUS");

            tokio::spawn(async move {
                let proxy = fdo::PropertiesProxy::builder(&connection)
                    .destination("org.freedesktop.ModemManager1")
                    .expect("Destination invalide")
                    .path("/org/freedesktop/ModemManager1/Modem/0")
                    .expect("Path invalide")
                    .build()
                    .await
                    .expect("Impossible de créer le proxy pour la propriété");

                while !token.is_cancelled() {
                    let signal_quality: OwnedValue = proxy
                        .get(
                            InterfaceName::try_from("org.freedesktop.ModemManager1.Modem")
                                .expect("Type invalide"),
                            "SignalQuality",
                        )
                        .await
                        .expect("Impossible de récupérer la valeur SignalQuality.");

                    let signal = <(u32, bool)>::try_from(signal_quality).unwrap_or((0, false));

                    println!("Signal: {}", signal.0);

                    let _ = db.send_modem(signal.0).await;
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
        if let Err(e) = db.reset_switch().await {
            println!("[SWITCH] Impossible de réinitialiser les switchs ({e})");
        }

        #[cfg(feature = "real-actuators")]
        {
            let switch = crate::actuators::switch::Switch::new();
            if let Err(e) = switch {
                println!("[SWITCH] Erreur lors de l'init des switchs: {}", e);
                return;
            }
            let mut switch = switch.unwrap();

            while !token.is_cancelled() {
                let stream = db.live_switch().await;

                match stream {
                    Ok(mut s) => {
                        while !token.is_cancelled() {
                            let sw = s.next().await;
                            if let Some(Ok(data)) = sw {
                                if data.data.esc { switch.start_esc() } else { switch.stop_esc() };
                            }
                        }
                    },
                    Err(e) => {
                        eprintln!("[SWITCH] Erreur lors de la création du live: {}", e);
                    }
                }
            }

            switch.stop_esc();
        }
    }

    // Controle analogique
    {
        let token = token.child_token();
        let db = db.clone();
        tokio::spawn(async move {
            #[cfg(feature = "real-actuators")]
            {
                let motor = crate::actuators::motor::Motor::new();
                if let Err(e) = motor {
                    println!("[CONTROL] Erreur lors de l'init moteur: {}", e);
                    return;
                }
                let mut motor = motor.unwrap();

                let steer = crate::actuators::steering::Steering::new();
                if let Err(e) = steer {
                    println!("[CONTROL] Erreur lors de l'init steering: {}", e);
                    return;
                }
                let mut steer = steer.unwrap();

                while !token.is_cancelled() {
                    let stream = db.live_control().await;

                    match stream {
                        Ok(mut s) => {
                            while !token.is_cancelled() {
                                let control =
                                    timeout(Duration::from_millis(DEAD_TIMEOUT), s.next()).await;
                                match control {
                                    Ok(data) => {
                                        if data.is_none() {
                                            continue;
                                        }

                                        let data = data.unwrap();
                                        match data {
                                            Ok(data) => {
                                                if data.action != surrealdb::Action::Update {
                                                    continue;
                                                }

                                                if let Err(e) = steer.set_steer(data.data.steer) {
                                                    eprintln!("[CONTROL] Erreur lors du contrôle de la direction: {}", e)
                                                }

                                                if let Err(e) = motor.set_speed(data.data.speed) {
                                                    eprintln!("[CONTROL] Erreur lors du contrôle moteur: {}", e)
                                                }
                                            }

                                            Err(e) => {
                                                eprintln!(
                                                    "[CONTROL] Erreur lors de l'update: {}",
                                                    e
                                                );
                                            }
                                        }
                                    }
                                    Err(_) => {
                                        eprintln!("[CONTROL] Update tardif des données...");
                                        let _ = motor.set_speed(0.0);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("[CONTROL] Erreur lors de la création du live: {}", e);
                        }
                    }
                }

                motor.safe_stop();
                steer.safe_stop();
            }

            #[cfg(feature = "fake-actuators")]
            {
                while !token.is_cancelled() {
                    let stream = db.live_control().await;

                    match stream {
                        Ok(mut s) => {
                            while !token.is_cancelled() {
                                let control =
                                    timeout(Duration::from_millis(DEAD_TIMEOUT), s.next()).await;
                                match control {
                                    Ok(data) => {
                                        if data.is_none() {
                                            continue;
                                        }

                                        let data = data.unwrap();
                                        match data {
                                            Ok(data) => {
                                                if data.action != surrealdb::Action::Update {
                                                    continue;
                                                }

                                                println!(
                                                    "[CONTROL] Steer: {} Speed: {}",
                                                    data.data.steer, data.data.speed
                                                );
                                            }

                                            Err(e) => {
                                                eprintln!(
                                                    "[CONTROL] Erreur lors de l'update: {}",
                                                    e
                                                );
                                            }
                                        }
                                    }
                                    Err(_) => {
                                        eprintln!("[CONTROL] Update tardif des données...");
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("[CONTROL] Erreur lors de la création du live: {}", e);
                        }
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
                // token.cancel();
            },
            _ = signal::ctrl_c() => {
                println!("Signal de contrôle C reçu");
                // token.cancel();
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
