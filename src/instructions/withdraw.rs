use pinocchio::{AccountView, Address, ProgramResult};

/// Process withdraw instruction
///
/// Accounts:
/// 0. `[signer]` owner - the vault owner
/// 1. `[writable]` vault - the vault PDA account
/// 2. `[]` system_program
pub fn handler(_program_id: &Address, _accounts: &[AccountView], _amount: u64) -> ProgramResult {
    // TODO: implement withdraw logic

    Ok(())
}
