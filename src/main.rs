mod database;
mod sensors;

#[cfg(feature = "real-sensors")]
mod i2c;

use std::{sync::Arc, time::Duration};

use database::Database;
use futures::StreamExt;
use nmea_parser::{gnss::GgaQualityIndicator, ParsedMessage};
use tokio_util::sync::CancellationToken;

#[cfg(unix)]
use tokio::signal::unix::SignalKind;
use tokio::signal::{self};

#[tokio::main]
async fn main() {
    let token = CancellationToken::new();

    // Préparation de la base de donnée
    let db = match Database::new().await {
        Ok(db) => {
            dbg!("[DB] Connexion établie.");
            Arc::new(db)
        }
        Err(e) => {
            panic!("[DB] Erreur de connexion: {}", e);
        }
    };

    // GPS
    {
        let token = token.child_token();
        let mut reader = sensors::gps::Reader::new(token.clone()).unwrap();
        let db: Arc<Database> = db.clone();
        tokio::spawn(async move {
            while !token.is_cancelled() {
                if let Some(nmea) = reader.next().await {
                    match nmea {
                        ParsedMessage::Gga(gga) => {
                            // Passage à une structure gérable par SurrealDB
                            let db_gga = sensors::gps::DataGGA {
                                latitude: gga.latitude.unwrap_or(0.0),
                                longitude: gga.longitude.unwrap_or(0.0),
                                sat_in_view: gga.satellite_count.unwrap_or(0),
                                fix: gga.quality == GgaQualityIndicator::GpsFix,
                            };

                            let _ = db.send_gps_gga(db_gga).await;

                            // println!("Source:    {}",     gga.source);
                            // println!("Latitude:  {:.3}°", gga.latitude.unwrap_or(0.0));
                            // println!("Longitude: {:.3}°", gga.longitude.unwrap_or(0.0));
                            // println!("Satelites: {}", gga.satellite_count.unwrap_or(0));
                            // println!("Fix?: {}",  gga.quality == GgaQualityIndicator::GpsFix);
                            // println!("");
                        }
                        _ => {
                            // dbg!("Trame NMEA Inconnue.");
                        }
                    }
                }
            }
        });
    }

    // IMU
    {
        let token_imu = token.child_token();
        let mut imu = sensors::imu::reader::Reader::new(token_imu.clone()).unwrap();
        let db_imu = db.clone();
        tokio::spawn(async move {
            while !token_imu.is_cancelled() {
                if let Some(data) = imu.next().await {
                    // dbg!("Angles: {:?} T: {}°C", data.angles, data.temp);
                    let _ = db_imu.send_imu(data).await;
                }

                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        });
    }

    // Analog
    {
        let token = token.child_token();
        let mut reader = sensors::analog::reader::Reader::new(token.clone()).unwrap();
        let db = db.clone();
        tokio::spawn(async move {
            while !token.is_cancelled() {
                if let Some(data) = reader.next().await {
                    if let Ok(data) = data {
                        // dbg!("BATT: {} V", battery);
                        let _ = db.send_analog(data);
                    }
                }
            }
        });
    }

    // MAG
    {
        let token = token.child_token();
        let mut reader = sensors::mag::reader::Reader::new(token.clone()).unwrap();
        let db = db.clone();
        tokio::spawn(async move {
            while !token.is_cancelled() {
                if let Some(data) = reader.next().await {
                    if let Ok(data) = data {
                        //dbg!("[MAG] H: {} R: {}", data.heading, data.raw);
                        let _ = db.send_mag(data);
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
