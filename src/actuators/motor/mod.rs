use anyhow::anyhow;
use  rppal::pwm::{Channel, Polarity, Pwm};

pub struct Motor {
    pwm: Pwm,
    is_safe: bool,
}

const MOTOR_NEUTRAL: f64 = 0.07;
const MOTOR_MAX: f64 = 0.10;
const MOTOR_MAX_REV: f64 = 0.04;

impl Motor {
    pub fn new() -> anyhow::Result<Self> {
        println!("[MOTOR] Initialisation ...");
        let pwm = Pwm::with_frequency(Channel::Pwm0, 50.0, MOTOR_NEUTRAL, Polarity::Normal, true).map_err(|x| anyhow!(x))?;

        Ok(Motor { 
            pwm: pwm,
            is_safe: false,
        })
    }

    pub fn start_esc(&self, start: bool) -> anyhow::Result<()> {
        
        
        Ok(())
    }

    pub fn set_speed(&self, mut speed: f64) -> anyhow::Result<()> {
        if self.is_safe {
            return Ok(())
        }

        // Validation SYSTEMATIQUE des données.
        if speed < -1.0 || speed > 1.0  {
            speed = 0.0;    
        }

        // Calcul du duty cycle.
        let mut cycle = MOTOR_NEUTRAL;

        // Reverse
        if speed < 0.0 {
            cycle = MOTOR_NEUTRAL - ((-1.0 * speed) * (MOTOR_NEUTRAL - MOTOR_MAX_REV));
        }

        // Forward
        if speed > 0.0 {
            cycle = MOTOR_NEUTRAL + (speed * (MOTOR_MAX - MOTOR_NEUTRAL))
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
}
