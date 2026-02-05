# Compiles to uf2 file
#
# Requires: elf2uf2-rs
build:
  cargo build --release
  elf2uf2-rs target/thumbv6m-none-eabi/release/group0 out.uf2
