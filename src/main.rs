use std::process;

use starpsx::StarPSX;

fn main() {
    let mut psx = StarPSX::build().unwrap_or_else(|err| {
        eprintln!("Startup Error: {err}");
        process::exit(1);
    });

    println!("Starting emulator...");
    psx.run();
}
