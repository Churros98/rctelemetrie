use std::ops::Deref;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::Poll;
use std::thread;
use futures::Stream;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

#[cfg(feature = "fake-sensors")]
use rand::Rng;

#[cfg(feature = "real-sensors")]
use super::imu::IMU;

#[derive(Clone, Copy, Serialize, Deserialize)]
pub(crate) struct Data {
    pub angles: (f32, f32, f32),
    pub temp: f32,
}

pub(crate) struct Reader {
    data: Arc<Mutex<Data>>,
    token: CancellationToken,
}

impl Reader {
    pub(crate) fn new(token: CancellationToken) -> anyhow::Result<Self> {
        // Donnée du capteur
        let data: Arc<Mutex<Data>> = Arc::new(Mutex::new(Data { angles: (0.0, 0.0, 0.0), temp: 0 as f32 }));
        let data_thread = data.clone();

        let thread_token = token.clone();
        
        let reader = Reader { data, token };
        #[cfg(feature = "real-sensors")]
        {
            dbg!("[IMU] Démarrage du thread ...\n");
            thread::spawn(move || {
                if let Ok(mut imu) = IMU::new() {
                    while !thread_token.is_cancelled() {
                        if let Err(e) = imu.update() {
                            dbg!("[IMU] Erreur de calcul: {}\n", e);
                        }

                        // Défini la valeur
                        let angles = imu.get_angles();
                        let temp: f32 = imu.get_temp();

                        *data_thread.lock().unwrap() = Data { angles: (angles.x, angles.y, angles.z), temp };
                    }
                }
                dbg!("[IMU] Fin du thread.\n");
            });
        }

        #[cfg(feature = "fake-sensors")]
        {
            dbg!("[IMU] Démarrage du thread [FAKE] ...\n");
            thread::spawn(move || {
                let mut rng = rand::thread_rng();

                while !thread_token.is_cancelled() {
                    let x: f32 = rng.gen();
                    let y: f32 = rng.gen();
                    let z: f32 = rng.gen();
                    let t: f32 = rng.gen();

                    *data_thread.lock().unwrap() = Data { angles: (x, y, z), temp: t };
                }

                dbg!("[IMU] Fin du thread.\n");
            });
        }

        Ok(reader)
    }
}

impl Stream for Reader {
    type Item = Data;
    
    fn poll_next(self: Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
        if self.token.is_cancelled() {
            return Poll::Ready(None);
        }

        let data = self.data.lock().unwrap().deref().clone();
        Poll::Ready(Some(data))
    }
}
