#![allow(unused)]

use rppal::i2c::I2c;

pub trait I2CBit {
    fn ecriture_word(&self, command: u8, data: u8) -> anyhow::Result<()> ;
    fn lecture_word(&self, command: u8) -> anyhow::Result<u8> ;
    fn ecriture_dword(&self, command: u8, data: u16) -> anyhow::Result<()> ;
    fn lecture_dword(&self, command: u8) -> anyhow::Result<u16> ;
    fn ecriture_bit8(&self, command: u8, bit: u8, state: bool) -> anyhow::Result<()> ;
    fn lecture_bit8(&self, command: u8, bit: u8) -> anyhow::Result<bool> ;
    fn lecture_bits8(&self, command: u8, bit: u8, lenght: u8) -> anyhow::Result<u8> ;
    fn ecriture_bits8(&self, command: u8, bit: u8, lenght: u8, value_to_write: u8) -> anyhow::Result<()> ;
    fn lecture_bit16(&self, command: u8, bit: u8) -> anyhow::Result<bool> ;
    fn lecture_bits16(&self, command: u8, bit: u8, lenght: u8) -> anyhow::Result<u16> ;
    fn ecriture_bit16(&self, command: u8, bit: u8, state: bool) -> anyhow::Result<()> ;
    fn ecriture_bits16(&self, command: u8, bit: u8, lenght: u8, value_to_write: u16) -> anyhow::Result<()> ;
}

impl I2CBit for I2c {
    // Ecrit un octet (word) sur la position donnée d'un registre 8 bits
    fn ecriture_word(&self, command: u8, data: u8) -> anyhow::Result<()> {
        let mut buffer: &mut [u8] = &mut [data];
        self.block_write(command, &mut buffer).map_err(|e| anyhow::anyhow!(e))
    }

    /// Lecture d'un octet (word) sur la position donnée d'un registre 8 bits
    fn lecture_word(&self, command: u8) -> anyhow::Result<u8>  {
        let mut buffer: &mut [u8] = &mut [0];
        self.block_read(command, &mut buffer).map_err(|e| anyhow::anyhow!(e))?;
        Ok(buffer[0])
    }

    // Ecrit de 2 octets (dword) sur la position donnée d'un registre 16 bits
    fn ecriture_dword(&self, command: u8, data: u16) -> anyhow::Result<()>  {
        let mut buffer: &mut [u8] = &mut [0, 0];
        buffer[0] = (data >> 8) as u8;
        buffer[1] = data as u8;

        self.block_write(command, &mut buffer).map_err(|e| anyhow::anyhow!(e))
    }

    /// Lecture de 2 octets (dword) sur la position donnée d'un registre 16 bits
    fn lecture_dword(&self, command: u8) -> anyhow::Result<u16>  {
        let mut buffer: &mut [u8] = &mut [0, 0];
        self.block_read(command, &mut buffer).map_err(|e| anyhow::anyhow!(e))?;

        Ok( ((buffer[0] as u16) << 8) | buffer[1] as u16 )
    }

    /// Ecrit un bit sur la position donnée d'un registre 8 bits
    fn ecriture_bit8(&self, command: u8, bit: u8, state: bool) -> anyhow::Result<()>  {
        let mut buffer: &mut [u8] = &mut [0];
        self.block_read(command, &mut buffer).map_err(|e| anyhow::anyhow!(e))?;
        //println!("SET BIT: {:#04x} {} {}", command, bit, state);
        //println!("OLD: {:08b}", buffer[0]);
        if state {
            buffer[0] |= 1 << bit;
        } else {
            buffer[0] &= !(1 << bit);
        }
        //println!("NEW: {:08b}", buffer[0]);

        self.block_write(command, &mut buffer).map_err(|e| anyhow::anyhow!(e))
    }

    /// Ecrit un bit sur la position donnée d'un registre 16 bits
    fn ecriture_bit16(&self, command: u8, bit: u8, state: bool) -> anyhow::Result<()>  {
        let mut buffer: &mut [u8] = &mut [0, 0];
        self.block_read(command, &mut buffer).map_err(|e| anyhow::anyhow!(e))?;
        let mut data: u16 = ((buffer[0] as u16) << 8) | buffer[1] as u16;

        if state {
            data |= 1 << bit;
        } else {
            data &= !(1 << bit);
        }

        buffer[0] = (data >> 8) as u8;
        buffer[1] = data as u8;

        self.block_write(command, &mut buffer).map_err(|e| anyhow::anyhow!(e))
    }

    /// Lis un bit sur la position donnée d'un registre 8 bits
    fn lecture_bit8(&self, command: u8, bit: u8) -> anyhow::Result<bool>  {
        let mut buffer: &mut [u8] = &mut [0];
        self.block_read(command, &mut buffer).map_err(|e| anyhow::anyhow!(e))?;
        
        Ok((buffer[0] & (1 << bit)) == (1 << bit))
    }

    /// Lis un bit sur la position donnée d'un registre 16 bits
    fn lecture_bit16(&self, command: u8, bit: u8) -> anyhow::Result<bool>  {
        let mut buffer: &mut [u8] = &mut [0, 0];
        self.block_read(command, &mut buffer).map_err(|e| anyhow::anyhow!(e))?;
        let data: u16 = ((buffer[0] as u16) << 8) | buffer[1] as u16;

        Ok((data & (1 << bit)) == (1 << bit))
    }

    /// Lis un ensemble de bits sur une position donnée d'un registre 8 bits
    fn lecture_bits8(&self, command: u8, bit: u8, lenght: u8) -> anyhow::Result<u8>  {
        let mut buffer: &mut [u8] = &mut [0];
        self.block_read(command, &mut buffer).map_err(|e| anyhow::anyhow!(e))?;

        let filtre = ((1u8 << lenght) - 1) << bit;
     
        Ok((buffer[0] & filtre) >> bit)
    }

    /// Lis un ensemble de bits sur une position donnée d'un registre 16 bits
    fn lecture_bits16(&self, command: u8, bit: u8, lenght: u8) -> anyhow::Result<u16>  {
        let mut buffer: &mut [u8] = &mut [0, 0];
        self.block_read(command, &mut buffer).map_err(|e| anyhow::anyhow!(e))?;
        let data: u16 = ((buffer[0] as u16) << 8) | buffer[1] as u16;

        let filtre = ((1u16 << lenght) - 1) << bit;

        Ok((data & filtre) >> bit)
    }
    
    /// Ecrit un ensemble de bits sur une position donnée d'un registre 8 bits
    fn ecriture_bits8(&self, command: u8, bit: u8, lenght: u8, value_to_write: u8) -> anyhow::Result<()>  {
        let mut buffer: &mut [u8] = &mut [0];
        self.block_read(command, &mut buffer).map_err(|e| anyhow::anyhow!(e))?;

        //println!("SET BITS: C[{:#04x}] B[{}] L[{}] => {:08b} ({:#04x})", command, bit, lenght, value_to_write, value_to_write);
        //println!("OLD REG: {:08b}", buffer[0]);

        let filtre_nettoyage = !(((1u8 << lenght) - 1) << bit);
        //println!("FILTRE : {:08b}", filtre_nettoyage);

        buffer[0] &= filtre_nettoyage;
        buffer[0] |=  value_to_write << bit;
        //println!("NEW REG: {:08b}", buffer[0]);

        self.block_write(command, &mut buffer).map_err(|e| anyhow::anyhow!(e))
    }

    /// Ecrit un ensemble de bits sur une position donnée d'un registre 16 bits
    fn ecriture_bits16(&self, command: u8, bit: u8, lenght: u8, value_to_write: u16) -> anyhow::Result<()>  {
        let mut buffer: &mut [u8] = &mut [0, 0];
        self.block_read(command, &mut buffer).map_err(|e| anyhow::anyhow!(e))?;
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

        self.block_write(command, &mut buffer).map_err(|e| anyhow::anyhow!(e))?;
        Ok(())
    }
}