use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::Poll;
use std::thread;
use std::time::Duration;

use anyhow::anyhow;
use futures::Stream;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

#[cfg(feature = "fake-sensors")]
use rand::Rng;

#[cfg(feature = "real-sensors")]
use super::analog::Analog;

#[derive(Clone, Copy, Serialize, Deserialize)]
pub(crate) struct Data {
    pub battery: f32,
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
            dbg!("[ANALOG] Démarrage du thread ...");
            thread::spawn(move || {
                if let Ok(mut analog) = Analog::new() {
                    while !thread_token.is_cancelled() {
                        *data_thread.lock().unwrap() = analog.get_battery().map(|x| Data { battery: x });
                        thread::sleep(Duration::from_millis(100));
                    }
                }
                dbg!("[ANALOG] Fin du thread.");
            });
        }

        #[cfg(feature = "fake-sensors")]
        {
            dbg!("[ANALOG] Démarrage du thread [FAKE] ...");
            thread::spawn(move || {
                let mut rng = rand::thread_rng();

                while !thread_token.is_cancelled() {
                    let batt: f32 = rng.gen();
                    *data_thread.lock().unwrap() = Ok(Data { battery: batt });
                    thread::sleep(Duration::from_millis(100));
                }
                dbg!("[ANALOG] Fin du thread [FAKE].");
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
                Poll::Ready(Some(Err(anyhow!("ANALERR"))))
            },
        };

        data
    }
}
