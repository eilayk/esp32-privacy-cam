build:
    cargo build

flash:
    cargo espflash flash --monitor

flash-release:
    cargo espflash flash --release --monitor

fix:
    cargo fmt && cargo fix --allow-dirty --allow-staged