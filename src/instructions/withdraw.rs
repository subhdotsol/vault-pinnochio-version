use pinocchio::{AccountView, Address, ProgramResult};

use crate::state::vault::Vault;

/// Process withdraw instruction
///
/// Accounts:
/// 0. `[signer, writable]` owner
/// 1. `[writable]` vault PDA account
/// 2. `[]` system_program
pub fn handler(
    program_id: &Address,
    accounts: &[AccountView],
    amount: u64,
    _bump: u8,
) -> ProgramResult {
    let [owner, vault, _system_program] = accounts else {
        return Err(pinocchio::error::ProgramError::NotEnoughAccountKeys);
    };

    // Validate owner is signer
    assert!(owner.is_signer(), "Owner must be signer");

    // Validate vault is owned by our program
    assert!(
        vault.owned_by(program_id),
        "Vault not owned by this program"
    );

    // Validate vault discriminator and owner
    let vault_state = Vault::from_account(vault);
    assert!(vault_state.owner() == owner.address(), "Owner mismatch");

    // Check sufficient balance
    let current_amount = vault_state.amount();
    assert!(current_amount >= amount, "Insufficient vault balance");

    // Direct lamport manipulation instead of System Program Transfer.
    // The System Program refuses transfers from accounts that carry data,
    // so we directly debit/credit lamports. This is safe because the
    // vault is a PDA owned by our program.
    let vault_current_lamports = vault.lamports();
    let owner_current_lamports = owner.lamports();

    vault.set_lamports(
        vault_current_lamports
            .checked_sub(amount)
            .expect("Vault lamport underflow"),
    );
    owner.set_lamports(
        owner_current_lamports
            .checked_add(amount)
            .expect("Owner lamport overflow"),
    );

    // Update the stored amount
    // SAFETY: no active borrows of vault data at this point
    let data = unsafe { vault.borrow_unchecked_mut() };
    let new_amount = current_amount
        .checked_sub(amount)
        .expect("Withdraw underflow");
    data[Vault::AMOUNT_OFFSET..Vault::AMOUNT_OFFSET + 8].copy_from_slice(&new_amount.to_le_bytes());

    Ok(())
}
