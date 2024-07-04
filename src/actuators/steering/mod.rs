use anyhow::anyhow;
use  rppal::pwm::{Channel, Polarity, Pwm};

pub struct Steering {
    pwm: Pwm,
    is_safe: bool,
}

const STEER_MID: f64 = 0.076;
const STEER_LEFT: f64 = 0.088;
const STEER_RIGHT: f64 = 0.064;

impl Steering {
    pub fn new() -> anyhow::Result<Self> {
        println!("[STEERING] Initialisation ...");

        let pwm = Pwm::with_frequency(Channel::Pwm1, 50.0, STEER_MID, Polarity::Normal, true).map_err(|x| anyhow!(x))?;

        Ok(Steering {
            pwm: pwm,
            is_safe: false,
        })
    }

    pub fn set_steer(&self, mut steer: f64) -> anyhow::Result<()> {
        if self.is_safe {
            return Ok(())
        }

        // Validation SYSTEMATIQUE des données.
        if steer < -1.0 || steer > 1.0 {
            steer = 0.0;
        }

        // Calcul du duty cycle.

        let mut cycle = STEER_MID;

        // Gauche
        if steer < 0.0 {
            cycle = STEER_MID + ((-1.0 * steer) * (STEER_LEFT - STEER_MID));
        }

        // Droite
        if steer > 0.0 {
            cycle = STEER_MID - (steer * (STEER_MID - STEER_RIGHT))
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
}
