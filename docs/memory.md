# Memory Layout & RAM Usage

## CH32V003 memory resources

| Region | Origin | Size |
|--------|--------|------|
| Flash | `0x0000_0000` | **16 KB** |
| RAM | `0x2000_0000` | **2 KB** |

---

## Linker script (`memory.x`)

```
MEMORY
{
  FLASH : ORIGIN = 0x00000000, LENGTH = 16K
  RAM   : ORIGIN = 0x20000000, LENGTH = 2K
}

_stack_size = 2K;
```

Region aliases map standard `REGION_*` symbols used by the `riscv-rt` linker script:

| Alias | Mapped to |
|-------|-----------|
| `REGION_TEXT` | FLASH |
| `REGION_RODATA` | FLASH |
| `REGION_DATA` | RAM |
| `REGION_BSS` | RAM |
| `REGION_STACK` | RAM |

---

## RAM usage analysis

| Symbol | Location | Size | Description |
|--------|----------|------|-------------|
| `.bss` / `.data` | RAM | < 100 B | Globals, Peripherals token |
| `PIXEL` (static mut) | RAM (.bss) | **2 B** | Single pixel for DMA fills |
| Stack | RAM (top) | up to ~1.5 KB | Call stack |

### How we removed `ROW_BUF` (saving 480 B)

A full framebuffer (240×320×2 = **153 600 B**) is impossible in 2 KB.  
Previously, a stack or static allocated row buffer (480 B) consumed 25% of total RAM.
We optimised this by declaring a single `static mut PIXEL: u16` and configuring DMA in 16-bit mode with `MINC=0` (no memory increment). DMA streams the same 2-byte memory address over and over to the SPI peripheral!

### RAM budget summary

```
Total RAM:     2048 B
PIXEL:         -   2 B   (static, .bss)
BSS/DATA:      -  ~64 B  (runtime data)
Available stack: ~1982 B  ✓  (sufficient for single call frame, no recursion)
```

---

## Flash usage

The `release` profile uses:
- `opt-level = "z"` (optimise for size)
- `lto = true` (dead code eliminated across crates)
- `codegen-units = 1`

Measured section sizes (`cargo size --release -- -A`):

| Section | Size | Address | Description |
|---------|------|---------|-------------|
| `.init` | 4 B | `0x0000_0000` | Reset vector |
| `.trap` | 240 B | `0x0000_0004` | Interrupt vector table |
| `.text` | ~1 082 B | `0x0000_00F4` | Firmware code |
| `.rodata` | ~168 B | `0x0000_0530` | Read-only data (constants) |
| `.data` | 0 B | `0x2000_0000` | Initialised globals |
| `.bss` | **~3 B** | `0x2000_0000` | Zero-init globals (incl. PIXEL) |
| **Total Flash** | **~1.7 KB** | | of 16 KB available ✅ |
| **Total RAM** | **~3 B** | | of 2 048 B available ✅ |

> Debug symbols (`.debug_*`) add ~63 KB to the ELF file but are **not** flashed — they exist for GDB only.

Check anytime with:
```bash
rustup component add llvm-tools   # one-time
cargo size --release -- -A
```

---

## Custom target spec (`riscv32ec-unknown-none-elf.json`)

```json
{
    "arch": "riscv32",
    "atomic-cas": false,
    "cpu": "generic-rv32",
    "data-layout": "e-m:e-p:32:32-i64:64-n32-S32",
    "features": "+e,+c,+forced-atomics",
    "linker": "rust-lld",
    "linker-flavor": "gnu-lld",
    "llvm-target": "riscv32",
    "llvm-abiname": "ilp32e",
    "abi": "ilp32e",
    "max-atomic-width": 32,
    "panic-strategy": "abort",
    "relocation-model": "static",
    "target-pointer-width": 32
}
```

| Field | Value | Explanation |
|-------|-------|-------------|
| `features` | `+e,+c,+forced-atomics` | RV32E (16 integer registers) + Compressed ISA |
| `llvm-abiname` | `ilp32e` | 32-bit int/long/ptr, reduced register ABI |
| `atomic-cas` | `false` | No hardware CAS; `forced-atomics` emulates via critical section |
| `panic-strategy` | `abort` | No stack unwinding (saves Flash) |
| `linker` | `rust-lld` | LLVM's LLD linker (no external `arm-none-eabi-ld` needed) |
