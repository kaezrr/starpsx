use std::error::Error;

use cpu::Cpu;
use memory::Bus;

mod cpu;
mod memory;

struct Config {
    pub bios_path: String,
}

impl Config {
    fn build(args: &[String]) -> Result<Config, Box<dyn Error>> {
        if args.len() < 2 {
            return Err("missing bios path".into());
        }

        let bios_path = args[1].clone();
        Ok(Config { bios_path })
    }
}

pub struct StarPSX {
    cpu: Cpu,
    bus: Bus,
}

impl StarPSX {
    pub fn build() -> Result<Self, Box<dyn Error>> {
        let args: Vec<String> = vec!["..".into(), "./bios/SCPH1001.BIN".into()];

        let config = Config::build(&args)?;
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
}

fn check_for_tty_output(cpu: &Cpu) {
    let pc = cpu.pc & 0x1FFFFFFF;
    if (pc == 0xA0 && cpu.regs[9] == 0x3C) || (pc == 0xB0 && cpu.regs[9] == 0x3D) {
        let ch = cpu.regs[4] as u8 as char;
        print!("{ch}");
    }
}
