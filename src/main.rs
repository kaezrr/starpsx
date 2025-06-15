use std::{env, process};

use cpu::Cpu;
use memory::Bus;

mod cpu;
mod memory;

struct Config {
    pub bios_path: String,
}

impl Config {
    fn build(args: &[String]) -> Result<Config, &'static str> {
        if args.len() < 2 {
            return Err("missing bios path");
        }

        let bios_path = args[1].clone();
        Ok(Config { bios_path })
    }
}

fn main() {
    let args: Vec<String> = vec!["..".into(), "./bios/SCPH1001.BIN".into()];
    let config = Config::build(&args).unwrap_or_else(|err| {
        eprintln!("Error parsing arguments: {err}");
        process::exit(1);
    });

    let memory = Bus::build(config).unwrap_or_else(|err| {
        eprintln!("Error starting emulator: {err}");
        process::exit(1);
    });

    let mut cpu = Cpu::new(memory);

    loop {
        cpu.run_instruction();
    }
}
