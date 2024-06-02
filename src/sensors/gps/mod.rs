use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::Waker;
use std::task::Poll;
use std::thread;
use std::time::Duration;
use futures::Stream;
use tokio_util::sync::CancellationToken;
use nmea_parser::*;
use serde::{Serialize, Deserialize};

#[cfg(feature = "real-sensors")]
use rppal::uart::{Parity, Uart};

#[cfg(feature = "real-sensors")]
use std::path::Path;

#[derive(Clone, Copy, Serialize, Deserialize)]
pub(crate) struct DataGGA {
    pub latitude: f64,
    pub longitude: f64,
    pub fix: bool,
    pub sat_in_view: u8,
}

pub(crate) struct Reader {
    events: Arc<Mutex<VecDeque<ParsedMessage>>>,
    waker: Arc<Mutex<VecDeque<Waker>>>,
    token: CancellationToken,
}

impl Reader {
    pub(crate) fn new(token: CancellationToken) -> anyhow::Result<Self> {
            let mut parser = NmeaParser::new();

            // Contexte à réveiller
            let waker: Arc<Mutex<VecDeque<Waker>>> = Arc::new(Mutex::new(VecDeque::with_capacity(50)));
            let waker_reader = waker.clone();

            // Evenement reçu du capteur
            let events: Arc<Mutex<VecDeque<ParsedMessage>>> = Arc::new(Mutex::new(VecDeque::with_capacity(50)));
            let events_reader = events.clone();

            let thread_token = token.clone();

            #[cfg(feature = "real-sensors")]
            {
                let path = Path::new("/dev/ttyS0");
                let mut uart = Uart::with_path(path, 38400, Parity::None, 8, 1)?;
                thread::spawn(move || {
                    dbg!("[GPS] Initialisation ...\n");

                    let mut buffer: Vec<u8> = Vec::new();
                    while !thread_token.is_cancelled() {
                        while !buffer.contains(&b'\n') {
                            let current_char = &mut [0;255];
                            
                            match uart.read(current_char) {
                                Ok(size) => {
                                    if size == 0 {
                                        thread::sleep(Duration::from_millis(100));
                                        continue;
                                    }
                                    buffer.extend_from_slice(&current_char[0..size]);
                                },
                                Err(e) => {
                                    dbg!("[GPS] Erreur: {}\n", e);
                                }       
                            }
                        }
            
                        let trame = buffer.iter().position(|&x| x == '\n' as u8).map(|idx| {
                            let (left, right) = buffer.split_at(idx + 1);
                            let trame = left.to_vec();
                            buffer = right.to_vec();
                            trame
                        });
            
                        match trame {
                            Some(v) => {
                                let trame = String::from_utf8(v).unwrap_or(String::new());
                                if let Ok(sentence) = parser.parse_sentence(trame.as_str()) {
                                    events.lock().unwrap().push_back(sentence);

                                    // Réveil la task (si elle existe).
                                    let wake = waker.lock().unwrap().pop_front();
                                    if let Some(w) = wake {
                                        w.wake();
                                    }
                                }
                            },
                            None => {
                                dbg!("Trame non connue.\n");
                            },
                        }
                    }

                    dbg!("[GPS] Fin de réception des données ...\n");
                });
            }

            #[cfg(feature = "fake-sensors")]
            {
                thread::spawn(move || {
                    dbg!("[GPS] Initialisation [FAKE] ...\n");

                    while !thread_token.is_cancelled() {
                        if let Ok(sentence) = parser.parse_sentence("$GPGGA,123519,4807.038,N,01131.324,E,1,08,0.9,545.4,M,46.9,M, , *42") {
                            events.lock().unwrap().push_back(sentence);

                            // Réveil Tokio
                            let wake = waker.lock().unwrap().pop_front();
                            if let Some(w) = wake {
                                w.wake();
                            }
                        }

                        thread::sleep(Duration::from_millis(100));
                    }

                    dbg!("[GPS] Fin de réception [FAKE] ...\n");
                });
            }

            Ok(Self { events: events_reader, waker: waker_reader, token: token })
    }
}

impl Stream for Reader {
    type Item = ParsedMessage;
    
    fn poll_next(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
        if self.token.is_cancelled() {
            return Poll::Ready(None);
        }

        match self.events.lock().unwrap().pop_front() {
            Some(data) => Poll::Ready(Some(data.clone())),
            None => {
                self.waker.lock().unwrap().push_back(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}
