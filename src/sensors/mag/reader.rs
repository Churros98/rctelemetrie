use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::Poll;
use std::thread;
use std::time::Duration;
use anyhow::anyhow;
use futures::Stream;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

#[cfg(feature = "real-sensors")]
use super::hmc8553l::HMC8553L;

#[cfg(feature = "fake-sensors")]
use rand::Rng;

#[derive(Clone, Copy, Serialize, Deserialize)]
pub(crate) struct Data {
    pub raw: (i16, i16, i16),
    pub heading: f32,
}

pub(crate) struct Reader {
    data: Arc<Mutex<anyhow::Result<Data>>>,
    token: CancellationToken,
}

impl Reader {
    pub(crate) fn new(token: CancellationToken) -> anyhow::Result<Self> {
        // Donnée du capteur
        let data: Arc<Mutex<anyhow::Result<Data>>> = Arc::new(Mutex::new(Err(anyhow!("NOINIT"))));
        let data_thread = data.clone();

        let thread_token = token.clone();

        let reader = Reader { data, token };
        #[cfg(feature = "real-sensors")]
        {
            dbg!("[MAG] Démarrage du thread ...\n");
            thread::spawn(move || {
                if let Ok(mag) = HMC8553L::new() {
                    while !thread_token.is_cancelled() {
                        let heading = mag.get_heading();
                        let raw = mag.get_mag_axes_raw();

                        if let Err(e) = heading {
                            *data_thread.lock().unwrap() = Err(anyhow!(e));
                            continue;
                        }

                        if let Err(e) = raw {
                            *data_thread.lock().unwrap() = Err(anyhow!(e));
                            continue;
                        }

                        let data = Data { heading: heading.unwrap(), raw: raw.map(|x| (x.x, x.y, x.z)).unwrap() };
                        *data_thread.lock().unwrap() = Ok(data);

                        thread::sleep(Duration::from_millis(100));
                    }
                }

                dbg!("[MAG] Fin du thread.\n");
            });
        }

        #[cfg(feature = "fake-sensors")]
        {
            dbg!("[MAG] Démarrage du thread [FAKE] ...\n");
            thread::spawn(move || {
                let mut rng = rand::thread_rng();

                while !thread_token.is_cancelled() {
                    let x: i16 = rng.gen();
                    let y: i16 = rng.gen();
                    let z: i16 = rng.gen();
                    let h: f32 = rng.gen();

                    let data = Data { heading: h, raw: (x, y, z) };
                    *data_thread.lock().unwrap() = Ok(data);
                    thread::sleep(Duration::from_millis(100));
                }

                dbg!("[MAG] Fin du thread [FAKE].\n");
            });
        }

        Ok(reader)
    }
}

impl Stream for Reader {
    type Item = anyhow::Result<Data>;
    
    fn poll_next(self: Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
        if self.token.is_cancelled() {
            return Poll::Ready(None);
        }

        let data = match self.data.lock().unwrap().as_ref().copied() {
            Ok(val) => {
                Poll::Ready(Some(Ok(val)))
            },
            Err(_e) => {
                Poll::Ready(Some(Err(anyhow!("MAGERR"))))
            },
        };

        data
    }
}
