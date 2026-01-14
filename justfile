run level:
    RUST_LOG=starpsx={{level}},starpsx-core={{level}} \
    cargo run --

run-bios level:
    RUST_LOG=starpsx={{level}},starpsx-core={{level}} \
    cargo run -- --auto-run

run-game level game:
    RUST_LOG=starpsx={{level}},starpsx-core={{level}} \
    cargo run -- --auto-run "{{game}}"

run-bios-release:
    RUST_LOG=starpsx=info,starpsx-core=info \
    cargo run -- --auto-run

run-game-release game:
    RUST_LOG=starpsx=info,starpsx-core=info
    cargo run -- --auto-run "{{game}}"
