#[cfg(feature = "real-sensors")]
use rppal::gpio::Gpio;
use rppal::gpio::InputPin;
use core::f64;
use std::{f64::consts::PI, time::Instant};

const DIAMETRE_ROUE_ENTRAINEE: f64 = 0.6; // en mètre

pub(crate) struct Hall {
    hall_pin: InputPin,
    last_state: bool,
    last_time: Instant,
    speed: f64,
}

impl Hall {
    pub(crate) fn new() -> anyhow::Result<Self> {
        let gpio = Gpio::new()?;
        let hall_pin = gpio.get(17)?.into_input();

        println!("[HALL] Capteur vitesse prêt.");
        Ok(Hall { hall_pin, last_state: false, last_time: Instant::now(), speed: 0.0 })
    }

    /// Met à jour la valeur de la vitesse de rotation
    pub(crate) fn update(&mut self) {
        // Trigger au front montant
        if self.hall_pin.is_high() && !self.last_state {
            let current_time = Instant::now();

            // Durée pour laquelle 2 pi a été effectué sur la roue mesurée.
            let delta_time = current_time.duration_since(self.last_time);
            let delta_time_secs = delta_time.as_secs_f64(); // Convertir en f64 pour la division
            let vitesse_angulaire_roue_mesuree = (2.0 * f64::consts::PI) / delta_time_secs; // rad/s

            let vitesse_lineaire_roue_entrainee = vitesse_angulaire_roue_mesuree * (DIAMETRE_ROUE_ENTRAINEE / 2.0); // en m/s

            self.speed = vitesse_lineaire_roue_entrainee * 3.6; // en km/h

            self.last_time = current_time;
        }

        self.last_state = self.hall_pin.is_high();
    }

    /// Retourne la vitesse de rotation actuelle
    pub(crate) fn get_speed(&self) -> f64 {
        self.speed
    }
}
