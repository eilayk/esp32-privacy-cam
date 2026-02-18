build:
    cargo build

flash:
    cargo espflash flash --monitor

fix:
    cargo fmt && cargo fix --allow-dirty --allow-staged