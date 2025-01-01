use futures::Stream;
use nmea_parser::gnss::GgaQualityIndicator;
use nmea_parser::ParsedMessage;
use rppal::i2c::I2c;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::Poll;
use std::thread;
use tokio_util::sync::CancellationToken;

use crate::sensors::{analog, gps, imu, mag};

#[derive(Clone, Copy, Serialize, Deserialize)]
pub(crate) struct ModemData {
    pub quality: u32,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub(crate) struct MagData {
    pub raw: (i16, i16, i16),
    pub heading: f32,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub(crate) struct ImuData {
    pub angles: (f32, f32, f32),
    pub temp: f32,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub(crate) struct AnalogData {
    pub battery: f32,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub(crate) struct GpsData {
    pub speed_kmh: f64,
    pub latitude: f64,
    pub longitude: f64,
    pub satellites: u8,
    pub fix: bool,
    pub heading: f64,
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct SensorsData {
    pub mag: MagData,
    pub imu: ImuData,
    pub analog: AnalogData,
    pub gps: GpsData,
    pub time: u64,
}

pub(crate) struct Reader {
    data: Arc<Mutex<SensorsData>>,
    token: CancellationToken,
}

impl Reader {
    #[cfg(feature = "real-sensors")]
    pub(crate) fn new(token: CancellationToken) -> anyhow::Result<Self> {
        // Initalisation des données

        use std::time::{SystemTime, UNIX_EPOCH};
        let current_data = SensorsData {
            mag: MagData {
                raw: (0, 0, 0),
                heading: 0.0,
            },

            imu: ImuData {
                angles: (0.0, 0.0, 0.0),
                temp: 0.0,
            },

            analog: AnalogData {
                battery: 0.0
            },

            gps: GpsData {
                speed_kmh: 0.0,
                latitude: 0.0,
                longitude: 0.0,
                satellites: 0,
                fix: false,
                heading: 0.0,
            },
            time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        };

        // Gestion des données
        let data: Arc<Mutex<SensorsData>> = Arc::new(Mutex::new(current_data.clone()));
        let data_thread = data.clone();
        let thread_token = token.clone();
        let reader = Reader { data, token };

        // I2C
        let mut i2c_bus = I2c::new().expect("[I2C] Erreur de bus");

        println!("[CAPTEURS] Démarrage du thread ...\n");
        thread::spawn(move || {
            let mut current_data = current_data;

            let mut imu = imu::imu::IMU::new(&mut i2c_bus).expect("[IMU] Capteur non disponible.");
            let mag = mag::hmc8553l::HMC8553L::new(&mut i2c_bus).expect("[MAG] Capteur non disponible.");
            let mut analog = analog::analog::Analog::new(&mut  i2c_bus).expect("[ANALOG] Capteur indisponible.");
            let mut gps = gps::GPS::new().expect("[GPS] Capteur indisponible.");

            println!("[CAPTEURS] Initialisation terminée. Lecture des données.\n");

            while !thread_token.is_cancelled() {
                // Capteur: Magnétique
                let heading: Result<f32, anyhow::Error> = mag.get_heading(&mut i2c_bus);
                let raw = mag.get_mag_axes_raw(&mut i2c_bus);

                if heading.is_ok() || raw.is_ok() {
                    let heading: f32 = heading.unwrap();
                    current_data.mag = MagData {
                        heading,
                        raw: raw.map(|x| (x.x, x.y, x.z)).unwrap(),
                    };
                } else {
                    println!("[MAG] Erreur lors de la récupération des données.");
                }

                // Capteur: IMU
                imu.set_speed(current_data.gps.speed_kmh);
                if let Err(e) = imu.update(&mut i2c_bus) {
                    println!("[IMU] Erreur de calcul: {}\n", e);
                } else {
                    let angles = imu.get_angles();
                    let temp: f32 = imu.get_temp();

                    current_data.imu = ImuData {
                        angles: (angles.x, angles.y, angles.z),
                        temp,
                    }
                }

                // Capteur: Analog
                let battery = analog.get_battery(&mut i2c_bus);
                if let Err(e)  = battery {
                    println!("[ANALOG] Erreur: {}\n", e);
                } else {
                    current_data.analog.battery = battery.unwrap();
                }

                // Capteur: GPS
                let messages = gps.read();
                if let Err(e) = messages {
                    println!("[GPS] Erreur: {}\n", e);
                } else {
                    if let Some(messages) = messages.unwrap() {
                        for message in messages {
                            match message {
                                ParsedMessage::Gga(gga) => {
                                    // println!("Source:    {}",     gga.source);
                                    // println!("Latitude:  {:.3}°", gga.latitude.unwrap_or(0.0));
                                    // println!("Longitude: {:.3}°", gga.longitude.unwrap_or(0.0));
                                    // println!("Satelites: {}", gga.satellite_count.unwrap_or(0));
                                    // println!("Fix?: {}",  gga.quality == GgaQualityIndicator::GpsFix);
                                    current_data.gps.latitude = gga.latitude.unwrap_or(0.0);
                                    current_data.gps.longitude = gga.longitude.unwrap_or(0.0);
                                    current_data.gps.satellites = gga.satellite_count.unwrap_or(0);
                                    current_data.gps.fix = gga.quality == GgaQualityIndicator::GpsFix;

                                }
                                ParsedMessage::Vtg(vtg) => {
                                    current_data.gps.speed_kmh = vtg.sog_kph.unwrap_or(0.0);
                                    current_data.gps.heading = vtg.cog_true.unwrap_or(0.0);
                                }
                                _ => {
                                    // println!("Trame NMEA Inconnue.");
                                }
                            }
                        }
                    }
                }

                *data_thread.lock().unwrap() = current_data.clone();
            }


            println!("[CAPTEURS] Fin du thread.\n");
        });

        Ok(reader)
    }

    #[cfg(feature = "fake-sensors")]
    pub(crate) fn new(token: CancellationToken) -> anyhow::Result<Self> {
        // Initalisation des données
        let current_data = Data {
            mag: MagData {
                raw: (0, 0, 0),
                heading: 0.0,
            },

            imu: ImuData {
                angles: (0.0, 0.0, 0.0),
                temp: 0.0,
            },

            analog: AnalogData {
                battery: 0.0
            },

            gps: GpsData {
                speed_kmh: 0.0,
                latitude: 0.0,
                longitude: 0.0,
                satellites: 0,
                fix: false,
                heading: 0.0,
            }
        };

        // Gestion des données
        let data: Arc<Mutex<Data>> = Arc::new(Mutex::new(current_data.clone()));
        let data_thread = data.clone();
        let thread_token = token.clone();
        let reader = Reader { data, token };

        println!("[CAPTEURS] Démarrage du thread [FAKE] ...\n");
        thread::spawn(move || {
            let mut rng = rand::thread_rng();
            let mut current_data = current_data;

            while !thread_token.is_cancelled() {
                current_data.mag.heading = rng.gen();
                current_data.analog.battery = rng.gen();
                current_data.gps.speed_kmh = rng.gen();

                *data_thread.lock().unwrap() = current_data.clone();
            }

            println!("[CAPTEURS] Fin du thread [FAKE].\n");
        });

        Ok(reader)
    }
}

impl Stream for Reader {
    type Item = anyhow::Result<SensorsData>;

    fn poll_next(
        self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        if self.token.is_cancelled() {
            return Poll::Ready(None);
        }

        let data = self.data.lock().unwrap().clone();
        Poll::Ready(Some(Ok(data)))
    }
}
