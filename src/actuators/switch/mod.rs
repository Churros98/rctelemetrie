use rppal::gpio::{Gpio, OutputPin};

pub(crate) struct Switch {
    gpio: Gpio,
    esc_pin: OutputPin,
}

impl Switch {
    pub fn new() -> anyhow::Result<Switch> {
        let gpio = Gpio::new()?;
        let esc_pin = gpio.get(25)?.into_output();

        let switch = Switch {
            gpio,
            esc_pin
        };

        Ok(switch)
    }

    pub fn start_esc(&mut self) {
        self.esc_pin.set_high();
    }

    pub fn stop_esc(&mut self) {
        self.esc_pin.set_low();
    }

    pub fn get_esc(&self) -> bool {
        self.esc_pin.is_set_high()
    }
}