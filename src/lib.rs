use std::{error::Error, fs};

use cpu::Cpu;
use memory::Bus;

mod cpu;
mod dma;
mod memory;

pub struct Config {
    pub bios_path: String,
    pub exe_path: Option<String>,
}

impl Config {
    pub fn build(args: &[String]) -> Result<Config, Box<dyn Error>> {
        if args.len() < 2 {
            return Err("missing bios path".into());
        }

        let bios_path = args[1].clone();
        let exe_path = args.get(2).cloned();

        Ok(Config {
            bios_path,
            exe_path,
        })
    }
}

pub struct StarPSX {
    cpu: Cpu,
    bus: Bus,
}

impl StarPSX {
    pub fn build(config: &Config) -> Result<Self, Box<dyn Error>> {
        let bus = Bus::build(config)?;
        let cpu = Cpu::new();

        Ok(StarPSX { cpu, bus })
    }

    pub fn run(&mut self) -> ! {
        loop {
            self.cpu.run_instruction(&mut self.bus);
            check_for_tty_output(&self.cpu);
        }
    }

    pub fn sideload_exe(&mut self, filepath: &String) -> Result<(), Box<dyn Error>> {
        let exe = fs::read(filepath)?;
        while self.cpu.pc != 0x80030000 {
            self.cpu.run_instruction(&mut self.bus);
        }

        // Parse EXE header
        let init_pc = u32::from_le_bytes(exe[0x10..0x14].try_into().unwrap());
        let init_r28 = u32::from_le_bytes(exe[0x14..0x18].try_into().unwrap());
        let exe_addr = u32::from_le_bytes(exe[0x18..0x1C].try_into().unwrap()) & 0x1FFFFF;
        let exe_size = u32::from_le_bytes(exe[0x1C..0x20].try_into().unwrap());
        let init_sp = u32::from_le_bytes(exe[0x30..0x34].try_into().unwrap());

        // Copy EXE data to RAM
        self.bus.ram.bytes[exe_addr as usize..(exe_addr + exe_size) as usize]
            .copy_from_slice(&exe[2048..2048 + exe_size as usize]);

        self.cpu.regs[28] = init_r28;
        if init_sp != 0 {
            self.cpu.regs[29] = init_sp;
            self.cpu.regs[30] = init_sp;
        }

        self.cpu.pc = init_pc;

        Ok(())
    }
}

fn check_for_tty_output(cpu: &Cpu) {
    let pc = cpu.pc & 0x1FFFFFFF;
    if (pc == 0xA0 && cpu.regs[9] == 0x3C) || (pc == 0xB0 && cpu.regs[9] == 0x3D) {
        let ch = cpu.regs[4] as u8 as char;
        print!("{ch}");
    }
}
