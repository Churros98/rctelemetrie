#![allow(unused)]

// QMC5883L
pub const QMC5883L_MAG_ADDR: u16 = 0x0D;

pub const QMC5883L_X_L: u8 = 0x00;
pub const QMC5883L_X_H: u8 = 0x01;
pub const QMC5883L_Y_L: u8 = 0x02;
pub const QMC5883L_Y_H: u8 = 0x03;
pub const QMC5883L_Z_L: u8 = 0x04;
pub const QMC5883L_Z_H: u8 = 0x05;

pub const QMC5883L_INFO: u8 = 0x06;
pub const QMC5883L_SETTINGS: u8 = 0x09;
pub const QMC5883L_SETRESET: u8 = 0x0B;
pub const QMC5883L_CHIP_ID: u8 = 0x0D;

pub const QMC5883L_INFO_DRDY_BIT: u8 = 0;
pub const QMC5883L_INFO_OVL_BIT: u8 = 1;
pub const QMC5883L_INFO_DOR_BIT: u8 = 2;

pub const QMC5883L_SETTINGS_MODE_BIT: u8 = 0;
pub const QMC5883L_SETTINGS_ODR_BIT: u8 = 2;
pub const QMC5883L_SETTINGS_RNG_BIT: u8 = 4;
pub const QMC5883L_SETTINGS_OSR_BIT: u8 = 6;
pub const QMC5883L_SETTINGS_SIZE: u8 = 2;


// HMC8553L
pub const HMC8553L_MAG_ADDR: u16 = 0x1E;

pub const HMC8553L_CONF_A: u8 = 0x00;
pub const HMC8553L_CONF_B: u8 = 0x01;
pub const HMC8553L_MODE: u8 = 0x02;
pub const HMC8553L_X_H: u8 = 0x03;
pub const HMC8553L_X_L: u8 = 0x04;
pub const HMC8553L_Z_H: u8 = 0x05;
pub const HMC8553L_Z_L: u8 = 0x06;
pub const HMC8553L_Y_H: u8 = 0x07;
pub const HMC8553L_Y_L: u8 = 0x08;

