use rppal::gpio::{Gpio, OutputPin};

pub(crate) struct ESC {
    esc_pin: OutputPin,
}

impl ESC {
    pub (crate) fn new() -> anyhow::Result<ESC> {
        let gpio = Gpio::new()?;
        let esc_pin = gpio.get(25)?.into_output();

        let esc = ESC {
            esc_pin
        };

        Ok(esc)
    }

    pub (crate) fn start(&mut self) {
        println!("[SWITCH] Démarrage de l'ESC ...");
        self.esc_pin.set_high();
    }

    pub (crate) fn stop(&mut self) {
        println!("[SWITCH] Arrêt de l'ESC ...");
        self.esc_pin.set_low();
    }
}