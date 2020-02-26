# msectrax-firmware

Converts ADC inputs to error angles (in DAC units) and then implements an
integral controller to drive the galvos using DACs.

## Installation/Building

```
rustup target add thumbv7m-none-eabi
rustup component add llvm-tools-preview
cargo install cargo-binutils
make
```

## license

GPLv1

Other license conditions may be possible. Contract Andrew Straw.
