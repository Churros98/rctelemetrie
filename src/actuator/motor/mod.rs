use  rppal::pwm::Pwm;
use std::error::Error;
use std::fmt;
use bincode::{Decode, Encode};

#[derive(Clone, Copy, Encode, Decode)]
pub struct MotorData {
    speed: f64,
}

impl fmt::Display for MotorData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Speed: {}", self.speed)
    }
}

pub struct Motor {
    pwm: Pwm,
    is_safe: bool,
}

const MOTOR_NEUTRAL: f64 = 0.07;
const MOTOR_MAX: f64 = 0.10;
const MOTOR_MAX_REV: f64 = 0.04;

impl Motor {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        println!("[MOTOR] Initialisation ...");
        let pwm = Pwm::new(rppal::pwm::Channel::Pwm1);
        match pwm {
            Ok(pwm) => {
                pwm.set_frequency(50.0, 0.0)?;
                pwm.enable()?;
        
                let motor = Motor { 
                    pwm: pwm,
                    is_safe: false,
                };
        
                Ok(motor)
            },

            Err(e) => {
                println!("[MOTOR] ERREUR: {}", e.to_string());
                Err("MOTORINIT")?
            }
        }
    }

    pub fn update(&self, mut data: MotorData) -> Result<(), Box<dyn Error>> {
        if self.is_safe {
            return Ok(())
        }

        // Validation SYSTEMATIQUE des données.
        if data.speed < -1.0 || data.speed > 1.0  {
            data.speed = 0.0;    
        }

        // Calcul du duty cycle.

        let mut cycle = MOTOR_NEUTRAL;

        // Reverse
        if data.speed < 0.0 {
            cycle = MOTOR_NEUTRAL - ((-1.0 * data.speed) * (MOTOR_NEUTRAL - MOTOR_MAX_REV));
        }

        // Forward
        if data.speed > 0.0 {
            cycle = MOTOR_NEUTRAL + (data.speed * (MOTOR_MAX - MOTOR_NEUTRAL))
        }

        // Défini le nouveau duty cycle
        self.pwm.set_duty_cycle(cycle)?;
        Ok(())
    }

    pub fn safe_stop(&mut self) {
        let _ = self.pwm.set_duty_cycle(0.0);
        let _ = self.pwm.disable();
        self.is_safe = true;
    }

    pub fn empty() -> MotorData {
        MotorData { 
            speed: 0.0,
        }
    }
}
