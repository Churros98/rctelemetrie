use anyhow::anyhow;
use futures::Stream;
use rppal::i2c::I2c;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::Poll;
use std::thread;
use tokio_util::sync::CancellationToken;

#[cfg(feature = "real-sensors")]
use super::hmc8553l::HMC8553L;

#[cfg(feature = "fake-sensors")]
use rand::Rng;

#[cfg(feature = "fake-sensors")]
use std::time::Duration;

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
    #[cfg(feature = "real-sensors")]
    pub(crate) fn new(i2c: Arc<Mutex<I2c>>, token: CancellationToken) -> anyhow::Result<Self> {
        // Donnée du capteur
        let data: Arc<Mutex<anyhow::Result<Data>>> = Arc::new(Mutex::new(Err(anyhow!("NOINIT"))));
        let data_thread = data.clone();

        let thread_token = token.clone();

        let reader = Reader { data, token };

        println!("[MAG] Démarrage du thread ...\n");
        thread::spawn(move || {
            let i2c = i2c;

            let mag = { HMC8553L::new(&mut i2c.lock().unwrap()) };

            if let Ok(mag) = mag {
                while !thread_token.is_cancelled() {
                    let i2c = &mut i2c.lock().unwrap();
                    let heading = mag.get_heading(i2c);
                    let raw = mag.get_mag_axes_raw(i2c);

                    if let Err(e) = heading {
                        println!("ERROR HEADING: {}", e);
                        *data_thread.lock().unwrap() = Err(anyhow!(e));
                        continue;
                    }

                    if let Err(e) = raw {
                        *data_thread.lock().unwrap() = Err(anyhow!(e));
                        continue;
                    }

                    let heading = heading.unwrap();
                    let data = Data {
                        heading: heading,
                        raw: raw.map(|x| (x.x, x.y, x.z)).unwrap(),
                    };
                    *data_thread.lock().unwrap() = Ok(data);
                }
            }

            println!("[MAG] Fin du thread.\n");
        });

        Ok(reader)
    }

    #[cfg(feature = "fake-sensors")]
    pub(crate) fn new(token: CancellationToken) -> anyhow::Result<Self> {
        println!("[MAG] Démarrage du thread [FAKE] ...\n");
        thread::spawn(move || {
            let mut rng = rand::thread_rng();

            while !thread_token.is_cancelled() {
                let x: i16 = rng.gen();
                let y: i16 = rng.gen();
                let z: i16 = rng.gen();
                let h: f32 = rng.gen();

                let data = Data {
                    heading: h,
                    raw: (x, y, z),
                };
                *data_thread.lock().unwrap() = Ok(data);
                thread::sleep(Duration::from_millis(100));
            }

            println!("[MAG] Fin du thread [FAKE].\n");
        });
    }
}

impl Stream for Reader {
    type Item = anyhow::Result<Data>;

    fn poll_next(
        self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        if self.token.is_cancelled() {
            return Poll::Ready(None);
        }

        let data = match self.data.lock().unwrap().as_ref().copied() {
            Ok(val) => Poll::Ready(Some(Ok(val))),
            Err(_e) => Poll::Ready(Some(Err(anyhow!("MAGERR")))),
        };

        data
    }
}
