use  rppal::pwm::Pwm;
use std::error::Error;
use std::fmt;
use bincode::{Decode, Encode};

#[derive(Clone, Copy, Encode, Decode)]
pub struct SteeringData {
    steer: f64, // -1.0 G | 0M | D 1.0
}

impl fmt::Display for SteeringData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Steering: {}", self.steer)
    }
}

pub struct Steering {
    pwm: Pwm,
    is_safe: bool,
}

const STEER_MID: f64 = 0.076;
const STEER_LEFT: f64 = 0.088;
const STEER_RIGHT: f64 = 0.064;

impl Steering {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        println!("[STEERING] Initialisation ...");

        let pwm = Pwm::new(rppal::pwm::Channel::Pwm0);
        match pwm {
            Ok(pwm) => {
                pwm.set_frequency(50.0, 0.0)?;
                pwm.enable()?;
        
                let steer = Steering { 
                    pwm: pwm,
                    is_safe: false,
                };
        
                Ok(steer)
            }
            
            Err(e) => {
                println!("[STEERING] ERREUR: {}", e.to_string());
                Err("STEERINIT")?
            }
        }
    }

    pub fn update(&self, mut data: SteeringData) -> Result<(), Box<dyn Error>> {
        if self.is_safe {
            return Ok(())
        }

        // Validation SYSTEMATIQUE des données.
        if data.steer < -1.0 || data.steer > 1.0 {
            data.steer = 0.0;
        }

        // Calcul du duty cycle.

        let mut cycle = STEER_MID;

        // Gauche
        if data.steer < 0.0 {
            cycle = STEER_MID + ((-1.0 * data.steer) * (STEER_LEFT - STEER_MID));
        }

        // Droite
        if data.steer > 0.0 {
            cycle = STEER_MID - (data.steer * (STEER_MID - STEER_RIGHT))
        }

        // Calcul du duty cycle.
        self.pwm.set_duty_cycle(cycle)?;
        Ok(())
    }

    pub fn safe_stop(&mut self) {
        let _ = self.pwm.set_duty_cycle(0.0);
        let _ = self.pwm.disable();
        self.is_safe = true;
    }

    pub fn empty() -> SteeringData {
        SteeringData { 
            steer: 0.0,
        }
    }
}
