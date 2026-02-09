use pinocchio::{AccountView, Address, ProgramResult};

/// Process initialize instruction
///
/// Accounts:
/// 0. `[signer]` owner - the vault owner / payer
/// 1. `[writable]` vault - the vault PDA account
/// 2. `[]` system_program
pub fn handler(_program_id: &Address, _accounts: &[AccountView]) -> ProgramResult {
    // TODO: implement initialize logic
    //
    // 1. Parse accounts
    // 2. Validate signer
    // 3. Derive vault PDA and verify
    // 4. Create vault account (CPI to system program)
    // 5. Write discriminator + owner + initial amount (0) to vault data

    Ok(())
}
