run level:
    RUST_LOG=starpsx={{level}},starpsx-core={{level}} \
    cargo run --

run-bios level:
    RUST_LOG=starpsx={{level}},starpsx-core={{level}} \
    cargo run -- --auto-run

run-game level game:
    RUST_LOG=starpsx={{level}},starpsx-core={{level}} \
    cargo run -- --auto-run {{game}}
