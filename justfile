bios := "./stuff/SCPH1001.BIN"

run-vram level *args:
    RUST_LOG=starpsx=error,starpsx_core={{level}},starpsx_renderer={{level}} \
    cargo run --features full-vram -- {{bios}} {{args}}

run level *args:
    RUST_LOG=starpsx=error,starpsx_core={{level}},starpsx_renderer={{level}} \
    cargo run -- {{bios}} {{args}}

run-trace *args:
    RUST_LOG=starpsx=info,starpsx_core=trace,mem=trace \
    cargo run -- {{bios}} {{args}}

run-release *args:
    RUST_LOG=starpsx=info,starpsx_core=info \
    cargo run --release --features full-vram -- {{bios}} {{args}}

run-disasm *args:
    RUST_LOG=starpsx=info,starpsx_core=trace,cpu=trace \
    cargo run --release -- {{bios}} {{args}}
