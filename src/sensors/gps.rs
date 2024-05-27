use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::Waker;
use std::{path::Path, task::Poll};
use std::thread;
use std::time::Duration;
use futures::Stream;
use tokio_util::sync::CancellationToken;
use nmea_parser::*;

#[cfg(feature = "real-sensors")]
use rppal::uart::{Parity, Uart};

pub(crate) struct Reader {
    events: Arc<Mutex<VecDeque<ParsedMessage>>>,
    waker: Arc<Mutex<Option<Waker>>>,
    token: CancellationToken,
}

impl Reader {
    /// ATTENTION: Le reader doit être unique.
    pub(crate) fn new(token: CancellationToken) -> anyhow::Result<Self> {
            #[cfg(feature = "real-sensors")]
            {
                let path = Path::new("/dev/ttyS0");
                let mut uart = Uart::with_path(path, 38400, Parity::None, 8, 1)?;
            }

            let mut parser = NmeaParser::new();

            let waker: Arc<Mutex<Option<Waker>>> = Arc::new(Mutex::new(None));
            let waker_reader = waker.clone();

            let events: Arc<Mutex<VecDeque<ParsedMessage>>> = Arc::new(Mutex::new(VecDeque::with_capacity(200)));
            let events_reader = events.clone();

            let thread_token = token.clone();
            thread::spawn(move || {
                println!("[GPS] Initialisation ...");

                let buffer: Vec<u8> = Vec::new();
                while !thread_token.is_cancelled() {
                    #[cfg(feature = "real-sensors")]
                    {
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
                                    dbg!("[GPS] Erreur: {}", e);
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

                                    // Réveil Tokio
                                    let wake = waker.lock().unwrap().take();
                                    if let Some(w) = wake {
                                        
                                        w.wake();
                                    }
                                }
                            },
                            None => {
                                dbg!("Trame non connue.");
                            },
                        }
                    }
            
                    #[cfg(feature = "fake-sensors")]
                    {
                        if let Ok(sentence) = parser.parse_sentence("$GPGLL,4916.45,N,12311.12,W,225444,A") {
                            events.lock().unwrap().push_back(sentence);

                            // Réveil Tokio
                            let wake = waker.lock().unwrap().take();
                            if let Some(w) = wake {
                                w.wake();
                            }
                        }
                    }

                    thread::sleep(Duration::from_millis(100));
                }

                // Réveil Tokio une dernière fois
                let wake = waker.lock().unwrap().take();
                if let Some(w) = wake {
                    w.wake();
                }

                println!("[GPS] Fin de réception des données ...");
            });

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
                *self.waker.lock().unwrap() = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}
