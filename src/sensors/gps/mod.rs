use nmea_parser::*;

#[cfg(feature = "real-sensors")]
use rppal::uart::{Parity, Uart};

#[cfg(feature = "real-sensors")]
use std::path::Path;

pub(crate) struct GPS {
    uart: Uart,
    parser: NmeaParser,
    buffer: Vec<u8>,
}

impl GPS {
    pub(crate) fn new() -> anyhow::Result<Self> {
            let parser = NmeaParser::new();
            let path = Path::new("/dev/ttyS0");
            let uart = Uart::with_path(path, 38400, Parity::None, 8, 1)?;
            let buffer = Vec::new();

            println!("[GPS] Initialisation ...");
            Ok(GPS { uart, parser, buffer })
    }

    pub(crate) fn read(&mut self) -> anyhow::Result<Option<Vec<ParsedMessage>>> {
        // Lecture des données.
        let current_char = &mut [0;255];
        match self.uart.read(current_char) {
            Ok(size) => {
                if size > 0 {
                    self.buffer.extend_from_slice(&current_char[0..size]);
                }
            },
            Err(e) => {
                println!("[GPS] Erreur: {}\n", e);
            }       
        }

        // Traitement des messages.
        let mut trames = Vec::new();
        while self.buffer.contains(&b'\n') {
            let trame = self.buffer.iter().position(|&x| x == '\n' as u8).map(|idx| {
                let (left, right) = self.buffer.split_at(idx + 1);
                let trame = left.to_vec();
                self.buffer = right.to_vec();
                trame
            });

            match trame {
                Some(v) => {
                    let trame = String::from_utf8(v).unwrap_or(String::new());
                    if let Ok(sentence) = self.parser.parse_sentence(trame.as_str()) {
                        trames.push(sentence);
                    }
                },
                None => {
                    println!("[GPS] Trame non connue.\n");
                },
            }
        }

        if trames.len() == 0 {
            return Ok(Option::None)
        }

        Ok(Some(trames))
    }
}