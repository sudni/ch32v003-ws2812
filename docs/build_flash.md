# Build & Flash Guide

## Prerequisites

### 1. Rust nightly toolchain (pinned)

The project uses `rust-toolchain.toml` which auto-installs the correct toolchain:

```toml
[toolchain]
channel = "nightly-2026-04-01"
components = ["rust-src", "rust-analyzer"]
```

Rustup will apply this automatically when you run any `cargo` command inside the project directory.

### 2. Custom RISC-V target

The CH32V003 uses the `riscv32ec` ISA (E = embedded base, C = compressed), which is not a built-in Rust target.  
The file `riscv32ec-unknown-none-elf.json` in the project root provides the target specification.

Key fields:
```json
{
  "arch": "riscv32",
  "features": "+e,+c,+forced-atomics",
  "llvm-abiname": "ilp32e",
  "panic-strategy": "abort"
}
```

`build-std` is required (set in `.cargo/config.toml`) to compile `core` for this custom target.

### 3. WCH-Link programmer

Flash using `wlink` (WCH's official tool):
```bash
cargo install wlink
```

Or use `minichlink` (open-source alternative):
```bash
# https://github.com/cnlohr/ch32v003fun/tree/master/minichlink
```

---

## Build

```bash
# Debug build (default)
cargo build

# Release build (optimised for size — recommended for deployment)
cargo build --release
```

The binary is placed at:
```
target/riscv32ec-unknown-none-elf/debug/ch32v003-blinky
target/riscv32ec-unknown-none-elf/release/ch32v003-blinky
```

---

## Flash

### Using wlink

```bash
wlink flash target/riscv32ec-unknown-none-elf/release/ch32v003-blinky
```

### Using wlink with auto-reset

```bash
wlink flash --watch target/riscv32ec-unknown-none-elf/release/ch32v003-blinky
```

### Using OpenOCD (if configured for WCH-Link)

```bash
openocd -f interface/wch-link.cfg -f target/wch-riscv.cfg \
  -c "program target/riscv32ec-unknown-none-elf/release/ch32v003-blinky verify reset exit"
```

---

## Project Cargo.toml dependencies

```toml
[dependencies]
ch32v0     = { version = "0.2.0", features = ["rt", "ch32v003", "critical-section"] }
qingke-rt  = "0.1.9"
qingke     = { version = "0.1.9", features = ["critical-section-impl"] }
panic-halt = "0.2.0"
riscv      = "0.11.1"
```

| Crate | Purpose |
|-------|---------|
| `ch32v0` | Peripheral Access Crate (PAC) — register definitions for CH32V003 |
| `qingke-rt` | Runtime (entry point macro `#[qingke_rt::entry]`, interrupt table) |
| `qingke` | QingKe core support (critical section implementation) |
| `panic-halt` | Panic handler — halts on panic (no `std` unwinding) |
| `riscv` | Generic RISC-V support crate |

---

## Build profile settings

```toml
[profile.release]
opt-level     = "z"   # Optimise for size (Flash = 16 KB)
lto           = true  # Link-time optimisation
codegen-units = 1     # Single codegen unit for best LTO
debug         = true  # Keep debug symbols for GDB

[profile.dev]
opt-level = "z"       # Also size-optimised in dev (critical for 16 KB)
```

---

## Debugging with GDB

### 1. Start OpenOCD / WCH GDB server

```bash
# Using WCH's OpenOCD fork
openocd -f interface/wch-link.cfg -f target/wch-riscv.cfg
```

### 2. Connect GDB

```bash
riscv32-unknown-elf-gdb target/riscv32ec-unknown-none-elf/debug/ch32v003-blinky \
  -ex "target extended-remote :3333" \
  -ex "load" \
  -ex "monitor reset halt"
```

### 3. VS Code

Use the `.vscode/` configuration already present in the project (if configured with `cortex-debug` or `probe-rs`).

---

## Common issues

| Problem | Likely cause | Fix |
|---------|-------------|-----|
| `error: can't find crate for 'core'` | Missing `build-std` config | Check `.cargo/config.toml` for `build-std = ["core"]` |
| `proc macro ABI mismatch` | Stale build cache | `cargo clean && cargo build` |
| Display stays blank | RST or DC not connected | Verify hardware connections |
| Wrong colours (BGR/RGB) | BGR bit mismatch | Change `0x36` data from `0x48` to `0x40` |
| Build fails for target | Wrong toolchain | `rustup show` — ensure nightly-2026-04-01 |
