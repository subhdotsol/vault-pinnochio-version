use pinocchio::error::ProgramError;
use pinocchio::{AccountView, Address};

// =============================================================================
// Basic Account Checks
// =============================================================================

/// Check if the account is a signer
pub fn signer_check(account: &AccountView) -> Result<(), ProgramError> {
    if !account.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    Ok(())
}

/// Check if the account is owned by the given program
pub fn owner_check(account: &AccountView, owner: &Address) -> Result<(), ProgramError> {
    if !account.owned_by(owner) {
        return Err(ProgramError::IllegalOwner);
    }

    Ok(())
}
