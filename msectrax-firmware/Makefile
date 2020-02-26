TARGET_DIR=target_makefile

SRCS=src/main.rs Cargo.toml build.rs

default: f103rb
f103rb: $(TARGET_DIR)/thumbv7m-none-eabi/release/msectrax-firmware.bin
all: f103rb

.PHONY: default f103rb all

# f103rb
$(TARGET_DIR)/thumbv7m-none-eabi/release/msectrax-firmware.bin: $(SRCS)
	# install with:
	#     cargo install cargo-binutils
	#     rustup component add llvm-tools-preview
	CARGO_TARGET_DIR=$(TARGET_DIR) cargo build --release --target thumbv7m-none-eabi
	CARGO_TARGET_DIR=$(TARGET_DIR) cargo objcopy --bin msectrax-firmware --release --target thumbv7m-none-eabi -- -O binary $@

install-linux-f103rb: $(TARGET_DIR)/thumbv7m-none-eabi/release/msectrax-firmware.bin
	cp $< /media/astraw/NODE_F103RB/
