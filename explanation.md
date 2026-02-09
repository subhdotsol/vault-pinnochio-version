# Pinocchio Vault Program — Complete Explanation

> A Solana program written in **raw Rust** using the **Pinocchio** framework (a lightweight alternative to Anchor) that lets a user create a personal vault, deposit SOL into it, and withdraw SOL from it.

---

## Table of Contents

1. [High-Level Architecture](#1-high-level-architecture)
2. [File Map & Connections](#2-file-map--connections)
3. [Cargo.toml — Dependencies](#3-cargotoml--dependencies)
4. [src/lib.rs — Module Root](#4-srclibrs--module-root)
5. [src/entrypoint.rs — Program Entry](#5-srcentrypointrs--program-entry)
6. [src/processor.rs — Instruction Router](#6-srcprocessorrs--instruction-router)
7. [src/instructions/mod.rs — Enum & Dispatch](#7-srcinstructionsmodrs--enum--dispatch)
8. [src/instructions/initialize.rs — Initialize Handler](#8-srcinstructionsinitializrs--initialize-handler)
9. [src/instructions/deposit.rs — Deposit Handler](#9-srcinstructionsdepositrs--deposit-handler)
10. [src/instructions/withdraw.rs — Withdraw Handler](#10-srcinstructionswithdrawrs--withdraw-handler)
11. [src/state/vault.rs — Vault Account Layout](#11-srcstatevaultrs--vault-account-layout)
12. [src/utils/helpers.rs — Utility Checks](#12-srcutilshelpersrs--utility-checks)
13. [Dry Run: Full Lifecycle](#13-dry-run-full-lifecycle)

---

## 1. High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  Client (off-chain)                                         │
│  Builds TX → [discriminator byte | data payload]            │
└────────────────────────┬────────────────────────────────────┘
                         │ Transaction
                         ▼
┌─────────────────────────────────────────────────────────────┐
│  entrypoint.rs                                              │
│  process_instruction(program_id, accounts, data)            │
│       └──► Processor::process(...)                          │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│  processor.rs                                               │
│  VaultInstruction::unpack(data)  → enum variant             │
│  instruction.process(program_id, accounts)                  │
└────────────────────────┬────────────────────────────────────┘
                         │ match variant
           ┌─────────────┼──────────────┐
           ▼             ▼              ▼
    ┌────────────┐ ┌──────────┐ ┌───────────┐
    │ initialize │ │ deposit  │ │ withdraw  │
    │  handler   │ │ handler  │ │  handler  │
    └─────┬──────┘ └────┬─────┘ └─────┬─────┘
          │             │             │
          ▼             ▼             ▼
    ┌─────────────────────────────────────────┐
    │       state/vault.rs (Vault struct)     │
    │  Raw byte layout: discriminator|owner|  │
    │  amount reads/writes via pointer math   │
    └─────────────────────────────────────────┘
```

**Flow in one sentence:** Client sends a transaction → `entrypoint.rs` receives it → hands off to `processor.rs` → which deserializes the instruction byte into an enum variant → dispatches to the correct handler → handler reads/writes `Vault` state.

---

## 2. File Map & Connections

| File | Role | Depends On | Depended On By |
|------|------|------------|----------------|
| `Cargo.toml` | Declares crate metadata and deps (`pinocchio`, `pinocchio-system`) | — | All `.rs` files |
| `src/lib.rs` | Root module — declares all sub-modules | — | Cargo (crate root) |
| `src/entrypoint.rs` | Solana runtime entry point | `processor.rs` | Solana runtime |
| `src/processor.rs` | Unpacks & dispatches instructions | `instructions/mod.rs` | `entrypoint.rs` |
| `src/instructions/mod.rs` | `VaultInstruction` enum + `unpack` + `process` | `initialize.rs`, `deposit.rs`, `withdraw.rs` | `processor.rs` |
| `src/instructions/initialize.rs` | Creates the vault PDA account on-chain | `state/vault.rs`, `pinocchio-system` | `instructions/mod.rs` |
| `src/instructions/deposit.rs` | Transfers SOL from owner → vault | `state/vault.rs`, `pinocchio-system` | `instructions/mod.rs` |
| `src/instructions/withdraw.rs` | Transfers SOL from vault → owner (PDA-signed) | `state/vault.rs`, `pinocchio-system` | `instructions/mod.rs` |
| `src/state/mod.rs` | Re-exports `vault` module | `vault.rs` | `instructions/*.rs` |
| `src/state/vault.rs` | `Vault` struct (zero-copy, pointer-based layout) | `pinocchio` | All instruction handlers |
| `src/utils/mod.rs` | Re-exports `helpers` module | `helpers.rs` | (available for future use) |
| `src/utils/helpers.rs` | `signer_check`, `owner_check` | `pinocchio` | (available for future use) |

### Why are they connected this way?

The Solana runtime calls a single `process_instruction` function. From that single entry point, the code is **layered**:

1. **Entrypoint** — keeps the runtime contract minimal.
2. **Processor** — separates "how to parse" from "how to route".
3. **Instruction enum** — provides a type-safe, exhaustive match over all operations.
4. **Handlers** — each file handles one instruction, keeping logic focused and testable.
5. **State** — shared data layout used by all handlers; centralises the byte offsets so no handler hard-codes magic numbers.
6. **Utils** — reusable safety checks that any handler can call.

This is the **standard Solana native program pattern** (without Anchor).

---

## 3. Cargo.toml — Dependencies

```toml
[package]
name = "vault"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "lib"]

[dependencies]
pinocchio = { version = "0.10.2", features = ["cpi"] }
pinocchio-system = "0.5.0"
```

### What each thing does:

| Key | Purpose |
|-----|---------|
| `crate-type = ["cdylib", "lib"]` | `cdylib` compiles to a shared object (`.so`) that the Solana runtime can load. `lib` allows unit-test imports. |
| `pinocchio` | Lightweight Solana framework — gives `entrypoint!`, `AccountView`, `Address`, `ProgramResult`, CPI helpers. Much thinner than Anchor. |
| `features = ["cpi"]` | Enables cross-program invocation support (needed for `create_account`, `transfer`). |
| `pinocchio-system` | Wrappers around the System Program instructions (`CreateAccount`, `Transfer`). |
| `no-entrypoint` feature | When enabled, skips registering `process_instruction`. Useful for importing this crate as a library in tests without symbol conflicts. |

---

## 4. src/lib.rs — Module Root

```rust
pub mod entrypoint;
pub mod instructions;
pub mod processor;
pub mod state;
pub mod utils;
```

**What it does:** Declares all 5 sub-modules so Rust's module system can resolve `crate::processor`, `crate::state::vault`, etc.

**Returns:** Nothing — it's purely structural.

**Why it exists:** Rust requires a `lib.rs` (or `main.rs`) to define the module tree. Without these `pub mod` lines, no other file could reference another.

---

## 5. src/entrypoint.rs — Program Entry

```rust
use pinocchio::{AccountView, Address, ProgramResult, entrypoint};
use crate::processor::Processor;

entrypoint!(process_instruction);

fn process_instruction(
    program_id: &Address,
    accounts: &[AccountView],
    data: &[u8],
) -> ProgramResult {
    Processor::process(program_id, accounts, data)
}
```

### Line-by-line:

1. **`entrypoint!(process_instruction)`** — This macro registers `process_instruction` as the function the Solana runtime will call. It generates the low-level `entrypoint` symbol in the compiled `.so` file.
2. **`process_instruction`** receives:
   - `program_id` — the public key of this deployed program.
   - `accounts` — slice of all accounts passed in the transaction.
   - `data` — the raw instruction data bytes.
3. It immediately delegates to `Processor::process(...)`.

**Returns:** `ProgramResult` (alias for `Result<(), ProgramError>`). `Ok(())` means success; any `Err(...)` aborts the transaction.

**Why it exists:** Solana mandates exactly one entrypoint per program. This file keeps it clean — just a thin bridge to the processor.

---

## 6. src/processor.rs — Instruction Router

```rust
use pinocchio::{AccountView, Address, ProgramResult};
use crate::instructions::VaultInstruction;

pub struct Processor;

impl Processor {
    pub fn process(program_id: &Address, accounts: &[AccountView], data: &[u8]) -> ProgramResult {
        let instruction = VaultInstruction::unpack(data)?;
        instruction.process(program_id, accounts)
    }
}
```

### What happens:

1. **`VaultInstruction::unpack(data)?`** — Deserializes the raw `data` bytes into one of three enum variants: `Initialize`, `Deposit`, or `Withdraw`. If the bytes are malformed, it returns `ProgramError::InvalidInstructionData`.
2. **`instruction.process(program_id, accounts)`** — Calls the `process` method on the enum, which dispatches to the right handler.

**Returns:** `ProgramResult` — passes through whatever the handler returns.

**Why it exists:** Separates parsing from execution. The processor doesn't know *how* each instruction works — it just routes.

---

## 7. src/instructions/mod.rs — Enum & Dispatch

### The Enum

```rust
pub enum VaultInstruction {
    Initialize { bump: u8 },       // discriminator byte 0
    Deposit { amount: u64 },       // discriminator byte 1
    Withdraw { amount: u64, bump: u8 }, // discriminator byte 2
}
```

Each variant carries the data it needs. The **first byte** of `data` tells the program which instruction this is.

### `unpack(data)` — Instruction Deserializer

```
data layout for each discriminator:

Initialize (0):  [0x00, bump]              → 2 bytes minimum
Deposit    (1):  [0x01, amount_le_bytes]   → 9 bytes minimum (1 + 8)
Withdraw   (2):  [0x02, amount_le_bytes, bump] → 10 bytes minimum (1 + 8 + 1)
```

**Dry run of `unpack` for a Deposit of 1 SOL (1_000_000_000 lamports):**

```
data = [0x01, 0x00, 0xCA, 0x9A, 0x3B, 0x00, 0x00, 0x00, 0x00]
         │     └────────────────────────────────────────────┘
         │              1_000_000_000 in little-endian
         └─ discriminator = 1 → Deposit

Step 1: data[0] = 1  → match arm for Deposit
Step 2: data.len() = 9 ≥ 9  → OK
Step 3: u64::from_le_bytes(data[1..9]) = 1_000_000_000
Result: VaultInstruction::Deposit { amount: 1_000_000_000 }
```

### `process(...)` — Dispatcher

```rust
match self {
    Initialize { bump } => initialize::handler(program_id, accounts, bump),
    Deposit { amount }   => deposit::handler(program_id, accounts, amount),
    Withdraw { amount, bump } => withdraw::handler(program_id, accounts, amount, bump),
}
```

Simply calls the corresponding handler function and passes the parsed fields.

---

## 8. src/instructions/initialize.rs — Initialize Handler

### Purpose
Creates a new **Vault PDA account** on-chain, owned by this program, and writes the initial state (discriminator, owner, amount = 0).

### Accounts expected

| Index | Account | Permissions |
|-------|---------|-------------|
| 0 | `payer` (the user's wallet) | signer, writable |
| 1 | `vault` (the PDA to create) | writable |
| 2 | `system_program` | read-only |

### Step-by-step dry run

Assume: User wallet = `5xYz...`, program_id = `Prog...`, bump = `254`

```
1. Destructure accounts → [payer, vault, _system_program]
   - If < 3 accounts → return NotEnoughAccountKeys

2. assert!(payer.is_signer())
   - User must have signed the TX

3. Build PDA seeds:
   seeds = ["vault", payer.address(), [254]]
   These seeds + our program_id derive a deterministic address:
   PDA = findProgramAddress(["vault", 5xYz...], Prog...) with bump=254

4. create_account_with_minimum_balance_signed(vault, 48, program_id, payer, None, &signers)
   This CPI to the System Program:
   - Allocates 48 bytes of data for `vault`
   - Funds it with rent-exempt minimum lamports (deducted from payer)
   - Sets owner to `program_id` (our program)
   - Signs the CPI with the PDA seeds (the vault doesn't have a private key!)

5. Write initial data into vault's 48 bytes:
   ┌────────────────┬──────────────────────────────┬────────────────┐
   │ Bytes 0..8     │ Bytes 8..40                  │ Bytes 40..48   │
   │ Discriminator   │ Owner (payer pubkey)          │ Amount (0)     │
   │ "Vault!!!"      │ 5xYz...                       │ 0x00...00      │
   └────────────────┴──────────────────────────────┴────────────────┘

6. Return Ok(())
```

**What it returns:** `ProgramResult` → `Ok(())` on success.

**Why PDA seeds include the payer address:** This ensures each user gets their *own* unique vault. Two different users can both initialize a vault without collision because their addresses are part of the seed.

---

## 9. src/instructions/deposit.rs — Deposit Handler

### Purpose
Transfers SOL from the user's wallet **into** the vault PDA, and updates the stored amount.

### Accounts expected

| Index | Account | Permissions |
|-------|---------|-------------|
| 0 | `owner` (user wallet) | signer, writable |
| 1 | `vault` (PDA) | writable |
| 2 | `system_program` | read-only |

### Step-by-step dry run

Assume: owner = `5xYz...`, vault already initialized with amount = 0, depositing 2 SOL (2_000_000_000 lamports)

```
1. Destructure accounts → [owner, vault, _system_program]

2. assert!(owner.is_signer())  → Must have signed

3. assert!(vault.owned_by(program_id))
   → Vault account must be owned by our program (proves it was created by us)

4. Vault::from_account(vault)
   → Reads vault data, checks:
     a. data_len() == 48          → correct
     b. discriminator == "Vault!!!" → correct
   → Returns a Vault view

5. assert!(vault_state.owner() == owner.address())
   → The owner field in state must match the TX signer
   → Prevents someone else from depositing into YOUR vault
      (and thus gaining the right to change your vault's state)

6. Transfer CPI (System Program):
   from: owner (5xYz...)
   to:   vault PDA
   lamports: 2_000_000_000
   → This is a normal transfer because the OWNER is the signer
   → .invoke() — no PDA signing needed

7. Update stored amount:
   current_amount = 0 (read from bytes 40..48)
   new_amount = 0 + 2_000_000_000 = 2_000_000_000
   Write new_amount into bytes 40..48

8. Return Ok(())
```

**Key detail:** The deposit uses `.invoke()` (not `.invoke_signed()`) because the **owner** (a normal wallet with a private key) is the sender. No PDA signing required.

---

## 10. src/instructions/withdraw.rs — Withdraw Handler

### Purpose
Transfers SOL from the vault PDA **back** to the owner. This requires **PDA signing** because the vault doesn't have a private key.

### Accounts expected

| Index | Account | Permissions |
|-------|---------|-------------|
| 0 | `owner` (user wallet) | signer, writable |
| 1 | `vault` (PDA) | writable |
| 2 | `system_program` | read-only |

### Step-by-step dry run

Assume: owner = `5xYz...`, vault has amount = 2_000_000_000, withdrawing 500_000_000 lamports, bump = 254

```
1. Destructure accounts → [owner, vault, _system_program]

2. assert!(owner.is_signer())  → Must have signed

3. assert!(vault.owned_by(program_id))  → Program ownership check

4. Vault::from_account(vault)
   → Validates discriminator and length

5. assert!(vault_state.owner() == owner.address())
   → Only the original owner can withdraw

6. current_amount = vault_state.amount() → 2_000_000_000

7. assert!(2_000_000_000 >= 500_000_000)  → Sufficient balance ✓

8. Build PDA signer seeds:
   seeds = ["vault", owner.address(), [254]]
   signers = [Signer::from(seeds)]

9. Transfer CPI (System Program) — PDA-SIGNED:
   from: vault PDA
   to:   owner (5xYz...)
   lamports: 500_000_000
   → .invoke_signed(&signers) — the runtime verifies that the seeds
     + bump produce the vault's address. This proves the program
     "authorized" the transfer on behalf of the PDA.

10. Update stored amount:
    new_amount = 2_000_000_000 - 500_000_000 = 1_500_000_000
    Write new_amount into bytes 40..48

11. Return Ok(())
```

**Critical difference from deposit:** `.invoke_signed(&signers)` is used because the vault (a PDA) is the *sender*. PDAs have no private key; the program must prove it derived the address to "sign" on its behalf.

---

## 11. src/state/vault.rs — Vault Account Layout

### The Data Layout (48 bytes total)

```
 Offset    Size    Field           Description
 ──────    ────    ─────           ───────────
  0        8       discriminator   Magic bytes: [0x56,0x61,0x75,0x6c,0x74,0x21,0x21,0x21]
                                   (ASCII for "Vault!!!")
  8        32      owner           The public key of the wallet that created this vault
  40       8       amount          Total SOL deposited (u64, little-endian)
```

### The Struct

```rust
pub struct Vault(*const u8);  // A raw pointer to the account's data buffer
```

This is a **zero-copy** design: no deserialization into a Rust struct with owned fields. Instead, `Vault` holds a raw pointer to the account data and reads fields directly via pointer arithmetic.

### Methods

| Method | What It Does | Returns |
|--------|-------------|---------|
| `from_account_unchecked(account)` | Gets a raw pointer to the account data — no validation | `Vault` |
| `from_account(account)` | Checks `data_len == 48` and discriminator matches `"Vault!!!"`, then calls `from_account_unchecked` | `Vault` (or panics) |
| `discriminator()` | Reads bytes 0..8 | `[u8; 8]` |
| `owner()` | Reads bytes 8..40 as an `Address` (32-byte pubkey) | `&Address` |
| `amount()` | Reads bytes 40..48 as a `u64` (little-endian) | `u64` |

### Why `*const u8` instead of normal deserialization?

Performance. Solana programs have a **compute budget** (200k CUs by default). Copying 48 bytes into a Rust struct with `borsh` deserialization has overhead. With pointer-based reads:
- No allocation
- No copying
- Direct memory access

The tradeoff is `unsafe` code, but the `from_account` method provides a safe entry point by validating first.

### Why a discriminator?

Account data on Solana is just raw bytes. If someone passes a random account to our program, we need to verify it's actually a vault. The 8-byte discriminator (`"Vault!!!"`) acts as a type tag. It's written during `initialize` and checked during `deposit`/`withdraw`.

---

## 12. src/utils/helpers.rs — Utility Checks

```rust
pub fn signer_check(account: &AccountView) -> Result<(), ProgramError> {
    if !account.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    Ok(())
}

pub fn owner_check(account: &AccountView, owner: &Address) -> Result<(), ProgramError> {
    if !account.owned_by(owner) {
        return Err(ProgramError::IllegalOwner);
    }
    Ok(())
}
```

**What they do:** Reusable validation functions. Currently the instruction handlers use inline `assert!()` calls instead, but these helpers are available as a **more graceful** alternative (they return `Err` instead of panicking).

**Returns:** `Result<(), ProgramError>` — `Ok(())` if check passes, `Err(...)` if it fails.

**Why they exist:** In production, you'd prefer `Err(ProgramError::...)` over `assert!` because:
- Errors propagate cleanly.
- `assert!` causes an abort, which is harder to debug and gives less useful error codes.

---

## 13. Dry Run: Full Lifecycle

Below is a complete lifecycle scenario showing exactly what happens at each step.

### Phase 1: Initialize

```
Client builds TX:
  data = [0x00, 0xFE]     // discriminator=0 (Initialize), bump=254
  accounts = [
    { pubkey: UserWallet,     is_signer: true,  is_writable: true },
    { pubkey: VaultPDA,       is_signer: false, is_writable: true },
    { pubkey: SystemProgram,  is_signer: false, is_writable: false },
  ]

→ entrypoint.rs: process_instruction(ProgramId, accounts, [0x00, 0xFE])
  → processor.rs: VaultInstruction::unpack([0x00, 0xFE])
    → data[0] = 0 → Initialize { bump: 254 }
  → VaultInstruction::process(...)
    → initialize::handler(ProgramId, accounts, 254)
      → Validate signer ✓
      → Build seeds: ["vault", UserWallet, [254]]
      → CPI: CreateAccount (48 bytes, rent-exempt, owner = ProgramId)
      → Write: discriminator = "Vault!!!", owner = UserWallet, amount = 0
      → Ok(())

Result: Vault PDA now exists on-chain with 48 bytes of data.
```

### Phase 2: Deposit 3 SOL

```
Client builds TX:
  data = [0x01, 0x00, 0x94, 0x35, 0x77, 0x00, 0x00, 0x00, 0x00]
  //       │     └── 3_000_000_000 in little-endian ──────────┘
  //       └─── discriminator = 1 (Deposit)
  accounts = [UserWallet, VaultPDA, SystemProgram]

→ unpack → Deposit { amount: 3_000_000_000 }
→ deposit::handler(...)
  → Validate signer ✓
  → Validate vault owned by program ✓
  → Validate discriminator ✓
  → Validate vault_state.owner() == UserWallet ✓
  → CPI: Transfer 3_000_000_000 lamports from UserWallet → VaultPDA
  → Update amount: 0 + 3_000_000_000 = 3_000_000_000
  → Ok(())

Result: Vault now holds 3 SOL. Vault state amount = 3_000_000_000.
```

### Phase 3: Withdraw 1 SOL

```
Client builds TX:
  data = [0x02, 0x00, 0xCA, 0x9A, 0x3B, 0x00, 0x00, 0x00, 0x00, 0xFE]
  //       │     └── 1_000_000_000 in LE ──────────────────┘    │
  //       │                                                     └─ bump=254
  //       └─── discriminator = 2 (Withdraw)
  accounts = [UserWallet, VaultPDA, SystemProgram]

→ unpack → Withdraw { amount: 1_000_000_000, bump: 254 }
→ withdraw::handler(...)
  → Validate signer ✓
  → Validate vault owned by program ✓
  → Validate discriminator ✓
  → Validate vault_state.owner() == UserWallet ✓
  → Check: 3_000_000_000 >= 1_000_000_000 ✓
  → Build PDA seeds: ["vault", UserWallet, [254]]
  → CPI (PDA-signed): Transfer 1_000_000_000 from VaultPDA → UserWallet
  → Update amount: 3_000_000_000 - 1_000_000_000 = 2_000_000_000
  → Ok(())

Result: 1 SOL returned to user. Vault state amount = 2_000_000_000.
```

---

## Summary of Returns

| Function | Returns | On Error |
|----------|---------|----------|
| `process_instruction` | `ProgramResult` → `Ok(())` | Propagates any error up |
| `Processor::process` | `ProgramResult` | `InvalidInstructionData` from unpack |
| `VaultInstruction::unpack` | `Result<VaultInstruction, ProgramError>` | `InvalidInstructionData` |
| `VaultInstruction::process` | `ProgramResult` | Whatever the handler returns |
| `initialize::handler` | `ProgramResult` | Panics on assert, CPI errors |
| `deposit::handler` | `ProgramResult` | Panics on assert, CPI errors, overflow |
| `withdraw::handler` | `ProgramResult` | Panics on assert, CPI errors, underflow, insufficient balance |
| `Vault::from_account` | `Vault` (or panics) | Panics if length != 48 or bad discriminator |
| `Vault::owner()` | `&Address` | — (unsafe, no error) |
| `Vault::amount()` | `u64` | — (unsafe, no error) |
| `signer_check` | `Result<(), ProgramError>` | `MissingRequiredSignature` |
| `owner_check` | `Result<(), ProgramError>` | `IllegalOwner` |
