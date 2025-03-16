use std::time::Instant;

use anyhow::anyhow;
use rppal::pwm::{Channel, Polarity, Pwm};

use crate::config::Config;

pub struct Motor {
    pwm: Pwm,
    is_safe: bool,
    elapsed_time: Instant,
    last_error: f64,
    error_integral: f64,
    coef: [f64; 3],
}

const MAX_SPEED: f64 = 30.0; // en km/h

const MOTOR_NEUTRAL: f64 = 0.07;
const MOTOR_MAX: f64 = 0.10;
const MOTOR_MAX_REV: f64 = 0.04;

impl Motor {
    pub fn new(config: Config) -> anyhow::Result<Self> {
        println!("[MOTOR] Initialisation ...");
        let pwm = Pwm::with_frequency(Channel::Pwm0, 50.0, MOTOR_NEUTRAL, Polarity::Normal, true).map_err(|x| anyhow!(x))?;

        Ok(Motor {
            pwm: pwm,
            is_safe: false,
            elapsed_time: Instant::now(),
            last_error: 0.0,
            error_integral: 0.0,
            coef: [config.kp, config.ki, config.kd],
        })
    }

    /// Normalise la vitesse en fonction de la vitesse maximale
    pub fn normalize_speed(&self, speed: f64) -> f64 {
        speed / MAX_SPEED
    }

    /// Consigne de vitesse (via PID)
    pub fn set_speed(&mut self, wanted_speed: f64, sensor_speed: f64) -> anyhow::Result<f64> {
        // Récupération du temps passé entre les commandes
        let start_time = Instant::now();
        let elapsed_time = start_time.duration_since(self.elapsed_time);

        // Normalisation de la vitesse du capteur 
        let sensor_speed = self.normalize_speed(sensor_speed);

        // Calcul de l'erreur
        let error = wanted_speed - sensor_speed;

        // Calcul de l'intégrale
        self.error_integral = self.error_integral + (error * elapsed_time.as_secs_f64());

        // Calcul de la dérivée
        let derivative = (error - self.last_error) / elapsed_time.as_secs_f64();

        // Calcul de la commande
        let command = wanted_speed + (self.coef[0] * error + self.coef[1] * self.error_integral + self.coef[2] * derivative);

        // Définition de la vitesse du moteur
        let set_speed = self.set_speed_esc(command)?;

        self.elapsed_time = start_time;
        self.last_error = error;

        Ok(set_speed)
    }

    /// Défini la vitesse du moteur (commande ESC direct)
    fn set_speed_esc(&mut self, mut speed: f64) -> anyhow::Result<f64> {
        if self.is_safe {
            return Ok(0.0)
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
        Ok(speed)
    }

    pub fn safe_stop(&mut self) {
        let _ = self.pwm.set_duty_cycle(0.0);
        let _ = self.pwm.disable();
        self.is_safe = true;
    }
}
