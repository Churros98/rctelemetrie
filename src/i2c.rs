use rppal::i2c::I2c;
use std::error::Error;

pub trait I2CBit {
    fn ecriture_word(&self, command: u8, data: u8) -> Result<(), Box<dyn Error>>;
    fn lecture_word(&self, command: u8) -> Result<u8, Box<dyn Error>>;
    fn ecriture_dword(&self, command: u8, data: u16) -> Result<(), Box<dyn Error>>;
    fn lecture_dword(&self, command: u8) -> Result<u16, Box<dyn Error>>;
    fn ecriture_bit8(&self, command: u8, bit: u8, state: bool) -> Result<(), Box<dyn Error>>;
    fn lecture_bit8(&self, command: u8, bit: u8) -> Result<bool, Box<dyn Error>>;
    fn lecture_bits8(&self, command: u8, bit: u8, lenght: u8) -> Result<u8, Box<dyn Error>>;
    fn ecriture_bits8(&self, command: u8, bit: u8, lenght: u8, value_to_write: u8) -> Result<(), Box<dyn Error>>;
    fn lecture_bit16(&self, command: u8, bit: u8) -> Result<bool, Box<dyn Error>>;
    fn lecture_bits16(&self, command: u8, bit: u8, lenght: u8) -> Result<u16, Box<dyn Error>>;
    fn ecriture_bit16(&self, command: u8, bit: u8, state: bool) -> Result<(), Box<dyn Error>>;
    fn ecriture_bits16(&self, command: u8, bit: u8, lenght: u8, value_to_write: u16) -> Result<(), Box<dyn Error>>;
}

impl I2CBit for I2c {
    // Ecrit un octet (word) sur la position donnée d'un registre 8 bits
    fn ecriture_word(&self, command: u8, data: u8) -> Result<(), Box<dyn Error>> {
        let mut buffer: &mut [u8] = &mut [data];
        self.block_write(command, &mut buffer)?;
        Ok(())
    }

    /// Lecture d'un octet (word) sur la position donnée d'un registre 8 bits
    fn lecture_word(&self, command: u8) -> Result<u8, Box<dyn Error>> {
        let mut buffer: &mut [u8] = &mut [0];
        self.block_read(command, &mut buffer)?;
        Ok(buffer[0])
    }

    // Ecrit de 2 octets (dword) sur la position donnée d'un registre 8 bits
    fn ecriture_dword(&self, command: u8, data: u16) -> Result<(), Box<dyn Error>> {
        let mut buffer: &mut [u8] = &mut [0, 0];
        buffer[0] = (data >> 8) as u8;
        buffer[1] = data as u8;

        self.block_write(command, &mut buffer)?;
        Ok(())
    }

    /// Lecture de 2 octets (dword) sur la position donnée d'un registre 8 bits
    fn lecture_dword(&self, command: u8) -> Result<u16, Box<dyn Error>> {
        let mut buffer: &mut [u8] = &mut [0];
        self.block_read(command, &mut buffer)?;
        
        Ok(((buffer[0] as u16) << 8) | buffer[1] as u16)
    }

    /// Ecrit un bit sur la position donnée d'un registre 8 bits
    fn ecriture_bit8(&self, command: u8, bit: u8, state: bool) -> Result<(), Box<dyn Error>> {
        let mut buffer: &mut [u8] = &mut [0];
        self.block_read(command, &mut buffer)?;
        //println!("SET BIT: {:#04x} {} {}", command, bit, state);
        //println!("OLD: {:08b}", buffer[0]);
        if state {
            buffer[0] |= 1 << bit;
        } else {
            buffer[0] &= !(1 << bit);
        }
        //println!("NEW: {:08b}", buffer[0]);

        self.block_write(command, &mut buffer)?;
        Ok(())
    }

    /// Ecrit un bit sur la position donnée d'un registre 16 bits
    fn ecriture_bit16(&self, command: u8, bit: u8, state: bool) -> Result<(), Box<dyn Error>> {
        let mut buffer: &mut [u8] = &mut [0, 0];
        self.block_read(command, &mut buffer)?;
        let mut data: u16 = ((buffer[0] as u16) << 8) | buffer[1] as u16;

        if state {
            data |= 1 << bit;
        } else {
            data &= !(1 << bit);
        }

        buffer[0] = (data >> 8) as u8;
        buffer[1] = data as u8;

        self.block_write(command, &mut buffer)?;
        Ok(())
    }

    /// Lis un bit sur la position donnée d'un registre 8 bits
    fn lecture_bit8(&self, command: u8, bit: u8) -> Result<bool, Box<dyn Error>> {
        let mut buffer: &mut [u8] = &mut [0];
        self.block_read(command, &mut buffer)?;
        
        Ok((buffer[0] & (1 << bit)) == (1 << bit))
    }

    /// Lis un bit sur la position donnée d'un registre 16 bits
    fn lecture_bit16(&self, command: u8, bit: u8) -> Result<bool, Box<dyn Error>> {
        let mut buffer: &mut [u8] = &mut [0, 0];
        self.block_read(command, &mut buffer)?;
        let data: u16 = ((buffer[0] as u16) << 8) | buffer[1] as u16;

        Ok((data & (1 << bit)) == (1 << bit))
    }

    /// Lis un ensemble de bits sur une position donnée d'un registre 8 bits
    fn lecture_bits8(&self, command: u8, bit: u8, lenght: u8) -> Result<u8, Box<dyn Error>> {
        let mut buffer: &mut [u8] = &mut [0];
        self.block_read(command, &mut buffer)?;

        let filtre = ((1u8 << lenght) - 1) << bit;
     
        Ok((buffer[0] & filtre) >> bit)
    }

    /// Lis un ensemble de bits sur une position donnée d'un registre 16 bits
    fn lecture_bits16(&self, command: u8, bit: u8, lenght: u8) -> Result<u16, Box<dyn Error>> {
        let mut buffer: &mut [u8] = &mut [0, 0];
        self.block_read(command, &mut buffer)?;
        let data: u16 = ((buffer[0] as u16) << 8) | buffer[1] as u16;

        let filtre = ((1u16 << lenght) - 1) << bit;

        Ok((data & filtre) >> bit)
    }
    
    /// Ecrit un ensemble de bits sur une position donnée d'un registre 8 bits
    fn ecriture_bits8(&self, command: u8, bit: u8, lenght: u8, value_to_write: u8) -> Result<(), Box<dyn Error>> {
        let mut buffer: &mut [u8] = &mut [0];
        self.block_read(command, &mut buffer)?;

        //println!("SET BITS: C[{:#04x}] B[{}] L[{}] => {:08b} ({:#04x})", command, bit, lenght, value_to_write, value_to_write);
        //println!("OLD REG: {:08b}", buffer[0]);

        let filtre_nettoyage = !(((1u8 << lenght) - 1) << bit);
        //println!("FILTRE : {:08b}", filtre_nettoyage);

        buffer[0] &= filtre_nettoyage;
        buffer[0] |=  value_to_write << bit;
        //println!("NEW REG: {:08b}", buffer[0]);

        self.block_write(command, &mut buffer)?;
        Ok(())
    }

    /// Ecrit un ensemble de bits sur une position donnée d'un registre 16 bits
    fn ecriture_bits16(&self, command: u8, bit: u8, lenght: u8, value_to_write: u16) -> Result<(), Box<dyn Error>> {
        let mut buffer: &mut [u8] = &mut [0, 0];
        self.block_read(command, &mut buffer)?;
        let mut data: u16 = ((buffer[0] as u16) << 8) | buffer[1] as u16;

        //println!("SET BITS: C[{:#04x}] B[{}] L[{}] => {:08b} ({:#04x})", command, bit, lenght, value_to_write, value_to_write);
        //println!("OLD REG: {:08b}", buffer[0]);

        let filtre_nettoyage = !(((1u16 << lenght) - 1) << bit);
        //println!("FILTRE : {:08b}", filtre_nettoyage);

        data &= filtre_nettoyage;
        data |=  value_to_write << bit;
        //println!("NEW REG: {:08b}", buffer[0]);

        buffer[0] = (data >> 8) as u8;
        buffer[1] = data as u8;

        self.block_write(command, &mut buffer)?;
        Ok(())
    }
}