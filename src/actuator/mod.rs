use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::watch::Receiver;
use std::sync::atomic::Ordering;
use std::error::Error;
use std::time::SystemTime;
use std::fmt;
use bincode::{Decode, Encode};

use self::motor::Motor;
use self::steering::Steering;

pub mod motor;
pub mod steering;

const DEAD_TIMEOUT: u128 = 500; // 0.5sec

#[derive(Encode, Decode)]
pub struct ActuatorData {
    motor: motor::MotorData,
    steering: steering::SteeringData,
}

impl fmt::Display for ActuatorData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({} {})", self.motor, self.steering)
    }
}

pub struct Acurator {
    is_stop: Arc<AtomicBool>,
    rx: Receiver<ActuatorData>,
    motor: motor::Motor,
    steering: steering::Steering,
    last_update: SystemTime,
    dead_timeout: bool,
}
impl Acurator {
    pub fn new(is_stop: Arc<AtomicBool>, rx: Receiver<ActuatorData>) -> Result<Self, Box<dyn Error>> {
        println!("[ACTUATOR] Initialisation ...");
    
        let motor = Motor::new()?;
        let steering = Steering::new()?;

        let acurator = Acurator {
            is_stop: is_stop,
            rx: rx,
            motor: motor,
            steering: steering,
            last_update: SystemTime::now(),
            dead_timeout: false,
        };

        Ok(acurator)
    }

    pub async fn update(&mut self) -> Result<(), Box<dyn Error>> {
        println!("[ACTUATOR] Pilotage des moteurs ...");

        // Traite les données reçu et contrôle les actionneurs
        while !self.is_stop.load(Ordering::Relaxed) {
            let data = self.rx.borrow_and_update();

            // Défini les valeurs après réception d'une commande
            if data.has_changed() {
                let _ = self.steering.update(data.steering);
                let _ = self.motor.update(data.motor);

                self.last_update = SystemTime::now();
                if self.dead_timeout {
                    println!("[ACTUATOR] Reprise d'une situation normale.");
                    self.dead_timeout = false;
                }
            } else {
                // Pas de réponse après X ms ? Alors je mets tous sur IDLE.
                if self.last_update.elapsed().unwrap().as_millis() > DEAD_TIMEOUT {
                    if !self.dead_timeout {
                        println!("[ACTUATOR] Aucune réponse du serveur. IDLE.");
                        self.dead_timeout = true;
                    }

                    let _ = self.steering.update(Steering::empty());
                    let _ = self.motor.update(Motor::empty())?;
                }
            }
        }

        self.safe_stop();
        println!("[ACTUATOR] Fin de pilotage des moteurs ...");
        Ok(())
    }

    pub fn safe_stop(&mut self) {
        self.steering.safe_stop();
        self.motor.safe_stop();
    }

    pub fn empty() -> ActuatorData {
        ActuatorData {
            motor: Motor::empty(),
            steering: Steering::empty(),
        }
    }
}