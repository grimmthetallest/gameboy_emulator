pub mod interrupt;

use self::interrupt::Interrupt;

const BIOS: &'static [u8] = include_bytes!("bios.gb");
const INTERRUPT_ENABLE_INDEX: u16 = 0xFFFF;
const INTERRUPT_FLAGS_INDEX: u16 = 0xFF0F;

pub struct Memory {
    rom_banks: Vec<[u8; 0x4000]>,
    eram_banks: Vec<[u8; 0x2000]>,
    wram_banks: Vec<[u8; 0x1000]>,
    vram_banks: Vec<[u8; 0x2000]>,
    high_ram: [u8; 0x200],
    disable_bios: u8,
    selected_rom_bank: usize,
    selected_eram_bank: usize,
    selected_wram_bank: usize,
    selected_vram_bank: usize,
    pub scan_line: u8,
}

impl Memory {
    pub fn new() -> Memory {
        Memory {
            rom_banks: vec![[0; 0x4000]; 2],
            vram_banks: vec![[0; 0x2000]],
            eram_banks: vec![[0; 0x2000]],
            wram_banks: vec![[0; 0x1000]; 2],
            high_ram: [0; 0x200],
            disable_bios: 0,
            selected_rom_bank: 1,
            selected_vram_bank: 0,
            selected_eram_bank: 0,
            selected_wram_bank: 1,
            scan_line: 0,
        }
    }

    pub fn read_byte(&self, index: u16) -> u8 {
        let index = index as usize;

        match index {
            0x0000...0x00FF => {
                if self.disable_bios == 0 {
                    BIOS[index]
                } else {
                    self.rom_banks[0][index]
                }
            }
            0x0100...0x3FFF => self.rom_banks[0][index],
            0x4000...0x7FFF => self.rom_banks[self.selected_rom_bank][index - 0x4000],
            0x8000...0x9FFF => {
                // println!("reading address {}", index);
                self.vram_banks[self.selected_vram_bank][index - 0x8000]
            }
            0xA000...0xBFFF => self.eram_banks[self.selected_eram_bank][index - 0xA000],
            0xC000...0xCFFF => self.wram_banks[0][index - 0xC000],
            0xD000...0xDFFF => self.wram_banks[self.selected_wram_bank][index - 0xD000],
            // 0xE000...0xFDFF => self.wram_banks[0][index - 0xE000],
            0xFE00...0xFFFF => match index {
                0xFF44 => self.scan_line,
                0xFF50 => self.disable_bios,
                _ => self.high_ram[index - 0xFE00],
            },
            _ => 0,
        }
    }

    pub fn write_byte(&mut self, index: u16, value: u8) {
        let index = index as usize;

        match index {
            0x8000...0x9FFF => {
                // println!("writing {:X} to address {:X}", value, index);
                self.vram_banks[self.selected_vram_bank][index - 0x8000] = value;
            }
            0xA000...0xBFFF => self.eram_banks[self.selected_eram_bank][index - 0xA000] = value,
            0xC000...0xCFFF => self.wram_banks[0][index - 0xC000] = value,
            0xD000...0xDFFF => self.wram_banks[self.selected_wram_bank][index - 0xD000] = value,
            0xFE00...0xFFFF => match index {
                0xFF44 => self.scan_line = 0,
                0xFF50 => self.disable_bios = value,
                _ => self.high_ram[index - 0xFE00] = value,
            },
            _ => {}
        };
    }

    pub fn read_word(&self, index: u16) -> u16 {
        let low = self.read_byte(index) as u16;
        let high = self.read_byte(index + 1) as u16;
        (high << 8) + low
    }

    pub fn write_word(&mut self, index: u16, value: u16) {
        let high = (value >> 8) as u8;
        let low = value as u8;
        self.write_byte(index, low);
        self.write_byte(index + 1, high);
    }

    pub fn get_interrupt(&self) -> Option<Interrupt> {
        let interrupt_enable = self.read_byte(INTERRUPT_ENABLE_INDEX);
        let interrupt_flags = self.read_byte(INTERRUPT_FLAGS_INDEX);

        let iflag = interrupt_enable & interrupt_flags;

        match iflag.trailing_zeros() {
            1 => Some(Interrupt::Vblank),
            2 => Some(Interrupt::Lcd),
            3 => Some(Interrupt::Timer),
            4 => Some(Interrupt::Joypad),
            _ => None,
        }
    }

    pub fn request_interrupt(&mut self, interrupt: Interrupt) {
        let interrupt_enable = self.read_byte(INTERRUPT_ENABLE_INDEX);
        let mut interrupt_flag = self.read_byte(INTERRUPT_FLAGS_INDEX);
        let interrupt = interrupt as u8;

        if interrupt_enable & interrupt != 0 {
            interrupt_flag |= interrupt;
            self.write_byte(INTERRUPT_FLAGS_INDEX, interrupt_flag);
        }
    }

    pub fn remove_interrupt(&mut self, interrupt: Interrupt) {
        let mut interrupt_flag = self.read_byte(INTERRUPT_FLAGS_INDEX);
        interrupt_flag &= !(interrupt as u8);
        self.write_byte(INTERRUPT_FLAGS_INDEX, interrupt_flag);
    }

    pub fn get_tile_from_set1(&self, tile_id: u8) -> &[u8] {
        let index = tile_id as usize * 0x10;
        &self.vram_banks[self.selected_vram_bank][index..index + 0x10]
    }

    pub fn get_tile_from_set0(&self, tile_id: i8) -> &[u8] {
        let index = (0x1000 + tile_id as i32 * 0x10) as usize;
        &self.vram_banks[self.selected_vram_bank][index..index + 0x10]
    }

    /// loads the rom into the memory struct
    ///
    /// # Arguments
    ///
    /// * `rom` - the rom file as represented as a vector of bytes.
    ///           Consumes the rom files, since the function makes a copy of the rom into memory
    pub fn load_rom(&mut self, rom: Vec<u8>) {
        self.rom_banks[0].copy_from_slice(&rom[..0x4000]);
        self.rom_banks[1].copy_from_slice(&rom[0x4000..0x8000]);

        let mut i = 0x8000;
        while i < rom.len() {
            let mut bank = [0u8; 0x4000];
            if i + 0x4000 >= rom.len() {
                bank[..rom.len() - i].copy_from_slice(&rom[i..]);
            } else {
                bank.copy_from_slice(&rom[i..i + 0x4000]);
            }
            self.rom_banks.push(bank);
            i += 0x4000;
        }
    }
}

#[cfg(test)]
mod tests {
    use mmu::Memory;

    #[test]
    fn test_write_byte() {
        let mut memory = Memory::new();
        memory.write_byte(0xFF80, 1);

        assert_eq!(memory.read_byte(0xFF80), 1);
    }

    #[test]
    fn test_write_word() {
        let mut memory = Memory::new();
        memory.write_word(0xFF80, 0x1122);
        assert_eq!(memory.read_word(0xFF80), 0x1122);
    }

    // #[test]
    // fn test_echo() {
    //     let mut memory = Memory::new();
    //     memory.write_byte(0xC000, 1);
    //     assert_eq!(memory.read_byte(0xC000), 1);
    //     assert_eq!(memory.read_byte(0xE000), 1);
    // }

    #[test]
    fn test_get_tile_from_map_0() {
        let mut memory = Memory::new();
        let tile = [
            0x7F, 0x00, 0x7F, 0x00, 0x7F, 0x00, 0x7F, 0x00, 0x7F, 0x00, 0x7F, 0x00, 0x7F, 0x00,
            0x7F, 0x00,
        ];

        for (i, row) in tile.iter().enumerate() {
            memory.write_byte((0x9000 + i) as u16, *row);
        }

        let actual = memory.get_tile_from_set0(0);

        for i in 0..0x10 {
            assert_eq!(tile[i], actual[i]);
        }
    }

    #[test]
    fn test_load_rom() {
        let mut memory = Memory::new();
        let mut rom = vec![0; 0xE000];
        for (i, item) in rom.iter_mut().enumerate() {
            match i {
                0x0000...0x3FFF => *item = 0u8,
                0x4000...0x7FFF => *item = 1u8,
                0x8000...0xBFFF => *item = 2u8,
                0xC000...0xE000 => *item = 3u8,
                _ => {}
            };
        }

        memory.load_rom(rom);

        assert_eq!(memory.rom_banks.len(), 4);

        for (i, bank) in memory.rom_banks.iter().enumerate() {
            match i {
                0 => for byte in bank.iter() {
                    assert_eq!(*byte, 0);
                },
                1 => for byte in bank.iter() {
                    assert_eq!(*byte, 1);
                },
                2 => for byte in bank.iter() {
                    assert_eq!(*byte, 2);
                },
                3 => for (i, byte) in bank.iter().enumerate() {
                    if i < 0x2000 {
                        assert_eq!(*byte, 3);
                    } else {
                        assert_eq!(*byte, 0);
                    }
                },
                _ => {}
            };
        }
    }
}