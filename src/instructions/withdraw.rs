use pinocchio::{
    cpi::{Seed, Signer},
    AccountView, Address, ProgramResult,
};
use pinocchio_system::instructions::Transfer;

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
    bump: u8,
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

    // Build PDA signer seeds (bump provided by client)
    let bump_bytes = [bump];
    let seeds: [Seed; 3] = [
        Seed::from(b"vault" as &[u8]),
        Seed::from(owner.address().as_ref()),
        Seed::from(&bump_bytes as &[u8]),
    ];
    let signers = [Signer::from(seeds.as_slice())];

    // Transfer SOL from vault to owner (PDA signed)
    Transfer {
        from: vault,
        to: owner,
        lamports: amount,
    }
    .invoke_signed(&signers)?;

    // Update the stored amount
    // SAFETY: no active borrows of vault data at this point
    let data = unsafe { vault.borrow_unchecked_mut() };
    let new_amount = current_amount
        .checked_sub(amount)
        .expect("Withdraw underflow");
    data[Vault::AMOUNT_OFFSET..Vault::AMOUNT_OFFSET + 8].copy_from_slice(&new_amount.to_le_bytes());

    Ok(())
}
