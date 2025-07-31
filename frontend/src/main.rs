use std::process;

use starpsx_core::{Config, StarPSX};

fn main() {
    let config = Config::build().unwrap_or_else(|err| {
        eprintln!("Argument parse error: {err}");
        process::exit(1);
    });

    let mut psx = StarPSX::build(&config).unwrap_or_else(|err| {
        eprintln!("Startup error: {err}");
        process::exit(1);
    });

    if let Some(exe_path) = config.exe_path {
        psx.sideload_exe(&exe_path).unwrap_or_else(|err| {
            eprintln!("EXE loading error: {err}");
            process::exit(1);
        });
    }

    psx.run();
}
