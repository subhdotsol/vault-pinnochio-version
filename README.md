# Vault Program

A native Solana program built with [Pinocchio](https://crates.io/crates/pinocchio), a zero dependency, zero allocation Solana program framework. This program implements a simple vault that allows users to deposit and withdraw SOL.

## Overview

The Vault program provides three core operations:

1. **Initialize** a new vault account owned by the caller.
2. **Deposit** SOL into the vault.
3. **Withdraw** SOL from the vault.

All state is stored on chain using a zero copy account layout for maximum performance.

## Project Structure

```
vault/
  src/
    entrypoint.rs          Program entrypoint
    processor.rs           Instruction dispatcher
    lib.rs                 Module declarations
    instructions/
      mod.rs               Instruction enum, unpacking, and routing
      initialize.rs        Initialize vault handler
      deposit.rs           Deposit handler
      withdraw.rs          Withdraw handler
    state/
      mod.rs               State module declarations
      vault.rs             Vault account layout and accessors
    utils/
      mod.rs               Utility module declarations
      helpers.rs           Account validation helpers
```

## Account Layout

The vault account uses a fixed size, zero copy layout totaling 48 bytes:

| Field          | Offset | Size (bytes) | Type        |
|----------------|--------|--------------|-------------|
| Discriminator  | 0      | 8            | `[u8; 8]`   |
| Owner          | 8      | 32           | `Address`   |
| Amount         | 40     | 8            | `u64` (LE)  |

The discriminator is set to `[0x53, 0x74, 0x6b, 0x50, 0x6f, 0x6f, 0x6c, 0x21]`.

## Instruction Format

Instructions are serialized as a single byte discriminator followed by any required data:

| Discriminator | Instruction | Data                    |
|---------------|-------------|-------------------------|
| `0`           | Initialize  | None                    |
| `1`           | Deposit     | `amount: u64` (8 bytes) |
| `2`           | Withdraw    | `amount: u64` (8 bytes) |

All integer values are encoded in little endian byte order.

## Dependencies

| Crate      | Version | Purpose                                    |
|------------|---------|--------------------------------------------|
| pinocchio  | 0.10.2  | Zero dependency Solana program framework   |

## Building

```bash
cargo build
```

For the Solana BPF target:

```bash
cargo build-sbf
```

## License

This project is unlicensed and intended for educational purposes.
