use pinocchio::{
    cpi::{Seed, Signer},
    AccountView, Address, ProgramResult,
};
use pinocchio_system::create_account_with_minimum_balance_signed;

use crate::state::vault::{Vault, VAULT_DISCRIMINATOR};

/// Process initialize instruction
///
/// Accounts:
/// 0. `[signer, writable]` owner / payer
/// 1. `[writable]` vault PDA account
/// 2. `[]` system_program
pub fn handler(program_id: &Address, accounts: &[AccountView], bump: u8) -> ProgramResult {
    let [payer, vault, _system_program] = accounts else {
        return Err(pinocchio::error::ProgramError::NotEnoughAccountKeys);
    };

    // Validate payer is signer
    assert!(payer.is_signer(), "Payer must be signer");

    // Verify the vault PDA matches expected derivation
    // The client derives find_program_address off-chain and passes the bump
    let bump_bytes = [bump];
    let seeds: [Seed; 3] = [
        Seed::from(b"vault" as &[u8]),
        Seed::from(payer.address().as_ref()),
        Seed::from(&bump_bytes as &[u8]),
    ];
    let signers = [Signer::from(seeds.as_slice())];

    // Create the vault account (PDA signed)
    create_account_with_minimum_balance_signed(
        vault,
        Vault::LEN,
        program_id,
        payer,
        None,
        &signers,
    )?;

    // Write vault data
    // SAFETY: we just created this account, no active borrows
    let data = unsafe { vault.borrow_unchecked_mut() };

    // Write discriminator
    data[Vault::DISCRIMINATOR_OFFSET..Vault::DISCRIMINATOR_OFFSET + 8]
        .copy_from_slice(&VAULT_DISCRIMINATOR);

    // Write owner
    data[Vault::OWNER_OFFSET..Vault::OWNER_OFFSET + 32].copy_from_slice(payer.address().as_ref());

    // Write initial amount (0)
    data[Vault::AMOUNT_OFFSET..Vault::AMOUNT_OFFSET + 8].copy_from_slice(&0u64.to_le_bytes());

    Ok(())
}
