use pinocchio::{AccountView, Address, ProgramResult};
use pinocchio_system::instructions::Transfer;

use crate::state::vault::Vault;

/// Process deposit instruction
///
/// Accounts:
/// 0. `[signer, writable]` owner
/// 1. `[writable]` vault PDA account
/// 2. `[]` system_program
pub fn handler(program_id: &Address, accounts: &[AccountView], amount: u64) -> ProgramResult {
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

    // Transfer SOL from owner to vault
    Transfer {
        from: owner,
        to: vault,
        lamports: amount,
    }
    .invoke()?;

    // Update the stored amount
    // SAFETY: no active borrows of vault data at this point
    let data = unsafe { vault.borrow_unchecked_mut() };
    let current_amount = u64::from_le_bytes(
        data[Vault::AMOUNT_OFFSET..Vault::AMOUNT_OFFSET + 8]
            .try_into()
            .unwrap(),
    );
    let new_amount = current_amount
        .checked_add(amount)
        .expect("Deposit overflow");
    data[Vault::AMOUNT_OFFSET..Vault::AMOUNT_OFFSET + 8].copy_from_slice(&new_amount.to_le_bytes());

    Ok(())
}
