mod sensors;

#[cfg(feature = "real-sensors")]
mod i2c;

use futures::StreamExt;
use nmea_parser::{gnss::GgaQualityIndicator, ParsedMessage};
use tokio_util::sync::CancellationToken;

#[cfg(unix)]
use tokio::signal::unix::SignalKind;
use tokio::signal::{self};

#[tokio::main]
async fn main() {
    let token = CancellationToken::new();

    let mut gps = sensors::gps::Reader::new(token.child_token()).unwrap();

    let task1 = tokio::spawn(async move {
        loop {
            if let Some(nmea) = gps.next().await {
                match nmea {
                    ParsedMessage::Gga(gga) => {
                        println!("Source:    {}",     gga.source);
                        println!("Latitude:  {:.3}°", gga.latitude.unwrap_or(0.0));
                        println!("Longitude: {:.3}°", gga.longitude.unwrap_or(0.0));
                        println!("Satelites: {}", gga.satellite_count.unwrap_or(0));
                        println!("Fix?: {}", gga.quality == GgaQualityIndicator::GpsFix);
                        println!("");
                    },
                    _ => {
                        dbg!("Trame NMEA Inconnue.");
                    }
                }
            }
        }
    });
    
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

    tokio::try_join!(task1).unwrap();
}
