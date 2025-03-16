use rppal::gpio::{Gpio, OutputPin};

pub(crate) struct Switch {
    esc_pin: OutputPin,
}

impl Switch {
    pub fn new() -> anyhow::Result<Switch> {
        let gpio = Gpio::new()?;
        let esc_pin = gpio.get(25)?.into_output();

        let switch = Switch {
            esc_pin
        };

        Ok(switch)
    }

    pub fn start_esc(&mut self) {
        println!("[SWITCH] Démarrage de l'ESC ...");
        self.esc_pin.set_high();
    }

    pub fn stop_esc(&mut self) {
        println!("[SWITCH] Arrêt de l'ESC ...");
        self.esc_pin.set_low();
    }
}