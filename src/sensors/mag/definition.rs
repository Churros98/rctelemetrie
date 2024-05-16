use std::error::Error;

pub trait MagSensor {
    fn get_mag_x_raw(&self) -> Result<i16, Box<dyn Error>>;
    fn get_mag_y_raw(&self) -> Result<i16, Box<dyn Error>>;
    fn get_mag_z_raw(&self) -> Result<i16, Box<dyn Error>>;
    fn get_status(&self) -> u8;
}