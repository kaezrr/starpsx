use std::process;

use starpsx::{Config, StarPSX};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let config = Config::build(&args).unwrap_or_else(|err| {
        eprintln!("Argument parse error: {err}");
        process::exit(1);
    });

    let mut psx = StarPSX::build(&config).unwrap_or_else(|err| {
        eprintln!("Startup error: {err}");
        process::exit(1);
    });
    println!("Starting emulator...");

    if let Some(exe_path) = config.exe_path {
        psx.sideload_exe(&exe_path).unwrap_or_else(|err| {
            eprintln!("EXE loading error: {err}");
            process::exit(1);
        });
    }

    psx.run();
}
