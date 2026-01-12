bios := "./stuff/SCPH1001.BIN"

run level *args:
    RUST_LOG=starpsx={{level}},starpsx_core={{level}} \
    cargo run -- {{bios}} "{{args}}"

run-bios level *args:
    RUST_LOG=starpsx={{level}},starpsx_core={{level}} \
    cargo run --release -- {{bios}}

run-release *args:
    RUST_LOG=starpsx=info,starpsx_core=info \
    cargo run --release -- {{bios}} "{{args}}"

