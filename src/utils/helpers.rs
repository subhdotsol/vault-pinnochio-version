use pinocchio::{
    ProgramResult,
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    pubkey::find_program_address,
    sysvars::rent::Rent,
};
use pinocchio_associated_token_account::instructions::Create;
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::instructions::{InitializeAccount3, InitializeMint2};

/// Custom error wrapper for Pinocchio operations
pub enum PinocchioError {
    NotSigner,
    InvalidOwner,
    InvalidAccountData,
    InvalidAddress,
}

impl From<PinocchioError> for ProgramError {
    fn from(e: PinocchioError) -> Self {
        match e {
            PinocchioError::NotSigner => ProgramError::MissingRequiredSignature,
            PinocchioError::InvalidOwner => ProgramError::IllegalOwner,
            PinocchioError::InvalidAccountData => ProgramError::InvalidAccountData,
            PinocchioError::InvalidAddress => ProgramError::InvalidSeeds,
        }
    }
}

// =============================================================================
// Basic Account Checks
// =============================================================================

/// Check if the account is a signer
pub fn signer_check(account: &AccountInfo) -> Result<(), ProgramError> {
    if !account.is_signer() {
        return Err(PinocchioError::NotSigner.into());
    }

    Ok(())
}

/// Check if the account is owned by the system program
pub fn system_account_check(account: &AccountInfo) -> Result<(), ProgramError> {
    if !account.is_owned_by(&pinocchio_system::ID) {
        return Err(PinocchioError::InvalidOwner.into());
    }

    Ok(())
}

// =============================================================================
// SPL Token (Legacy) Helpers
// =============================================================================

/// Helper for SPL Token Mint accounts
pub struct Mint;

impl Mint {
    /// Check if the account is a valid SPL Token Mint
    pub fn check(account: &AccountInfo) -> Result<(), ProgramError> {
        if !account.is_owned_by(&pinocchio_token::ID) {
            return Err(PinocchioError::InvalidOwner.into());
        }

        if account.data_len() != pinocchio_token::state::Mint::LEN {
            return Err(PinocchioError::InvalidAccountData.into());
        }

        Ok(())
    }

    /// Initialize a new SPL Token Mint
    pub fn init(
        account: &AccountInfo,
        payer: &AccountInfo,
        decimals: u8,
        mint_authority: &[u8; 32],
        freeze_authority: Option<&[u8; 32]>,
    ) -> ProgramResult {
        // Get required lamports for rent
        let lamports = Rent::get()?.minimum_balance(pinocchio_token::state::Mint::LEN);

        // Fund the account with the required lamports
        CreateAccount {
            from: payer,
            to: account,
            lamports,
            space: pinocchio_token::state::Mint::LEN as u64,
            owner: &pinocchio_token::ID,
        }
        .invoke()?;

        InitializeMint2 {
            mint: account,
            decimals,
            mint_authority,
            freeze_authority,
        }
        .invoke()
    }

    /// Initialize a new SPL Token Mint if it doesn't already exist
    pub fn init_if_needed(
        account: &AccountInfo,
        payer: &AccountInfo,
        decimals: u8,
        mint_authority: &[u8; 32],
        freeze_authority: Option<&[u8; 32]>,
    ) -> ProgramResult {
        match Self::check(account) {
            Ok(_) => Ok(()),
            Err(_) => Self::init(account, payer, decimals, mint_authority, freeze_authority),
        }
    }
}

/// Helper for SPL Token Account
pub struct Token;

impl Token {
    /// Check if the account is a valid SPL Token Account
    pub fn check(account: &AccountInfo) -> Result<(), ProgramError> {
        if !account.is_owned_by(&pinocchio_token::ID) {
            return Err(PinocchioError::InvalidOwner.into());
        }

        if account
            .data_len()
            .ne(&pinocchio_token::state::TokenAccount::LEN)
        {
            return Err(PinocchioError::InvalidAccountData.into());
        }

        Ok(())
    }

    /// Initialize a new SPL Token Account
    pub fn init(
        account: &AccountInfo,
        mint: &AccountInfo,
        payer: &AccountInfo,
        owner: &[u8; 32],
    ) -> ProgramResult {
        // Get required lamports for rent
        let lamports = Rent::get()?.minimum_balance(pinocchio_token::state::TokenAccount::LEN);

        // Fund the account with the required lamports
        CreateAccount {
            from: payer,
            to: account,
            lamports,
            space: pinocchio_token::state::TokenAccount::LEN as u64,
            owner: &pinocchio_token::ID,
        }
        .invoke()?;

        // Initialize the Token Account
        InitializeAccount3 {
            account,
            mint,
            owner,
        }
        .invoke()
    }

    /// Initialize a new SPL Token Account if it doesn't already exist
    pub fn init_if_needed(
        account: &AccountInfo,
        mint: &AccountInfo,
        payer: &AccountInfo,
        owner: &[u8; 32],
    ) -> ProgramResult {
        match Self::check(account) {
            Ok(_) => Ok(()),
            Err(_) => Self::init(account, mint, payer, owner),
        }
    }
}

// =============================================================================
// Token 2022 Helpers
// =============================================================================

/// Token 2022 Program ID: TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb
pub const TOKEN_2022_PROGRAM_ID: [u8; 32] = [
    0x06, 0xdd, 0xf6, 0xe1, 0xee, 0x75, 0x8f, 0xde, 0x18, 0x42, 0x5d, 0xbc, 0xe4, 0x6c, 0xcd, 0xda,
    0xb6, 0x1a, 0xfc, 0x4d, 0x83, 0xb9, 0x0d, 0x27, 0xfe, 0xbd, 0xf9, 0x28, 0xd8, 0xa1, 0x8b, 0xfc,
];

const TOKEN_2022_ACCOUNT_DISCRIMINATOR_OFFSET: usize = 165;
pub const TOKEN_2022_MINT_DISCRIMINATOR: u8 = 0x01;
pub const TOKEN_2022_TOKEN_ACCOUNT_DISCRIMINATOR: u8 = 0x02;

/// Helper for Token 2022 Mint accounts
pub struct Mint2022;

impl Mint2022 {
    /// Check if the account is a valid Token 2022 Mint
    pub fn check(account: &AccountInfo) -> Result<(), ProgramError> {
        if !account.is_owned_by(&TOKEN_2022_PROGRAM_ID) {
            return Err(PinocchioError::InvalidOwner.into());
        }

        let data = account.try_borrow_data()?;

        if data.len().ne(&pinocchio_token::state::Mint::LEN) {
            if data.len().le(&TOKEN_2022_ACCOUNT_DISCRIMINATOR_OFFSET) {
                return Err(PinocchioError::InvalidAccountData.into());
            }

            if data[TOKEN_2022_ACCOUNT_DISCRIMINATOR_OFFSET].ne(&TOKEN_2022_MINT_DISCRIMINATOR) {
                return Err(PinocchioError::InvalidAccountData.into());
            }
        }

        Ok(())
    }

    /// Initialize a new Token 2022 Mint
    pub fn init(
        account: &AccountInfo,
        payer: &AccountInfo,
        decimals: u8,
        mint_authority: &[u8; 32],
        freeze_authority: Option<&[u8; 32]>,
    ) -> ProgramResult {
        // Get required lamports for rent
        let lamports = Rent::get()?.minimum_balance(pinocchio_token::state::Mint::LEN);

        // Fund the account with the required lamports
        CreateAccount {
            from: payer,
            to: account,
            lamports,
            space: pinocchio_token::state::Mint::LEN as u64,
            owner: &TOKEN_2022_PROGRAM_ID,
        }
        .invoke()?;

        InitializeMint2 {
            mint: account,
            decimals,
            mint_authority,
            freeze_authority,
        }
        .invoke()
    }

    /// Initialize a new Token 2022 Mint if it doesn't already exist
    pub fn init_if_needed(
        account: &AccountInfo,
        payer: &AccountInfo,
        decimals: u8,
        mint_authority: &[u8; 32],
        freeze_authority: Option<&[u8; 32]>,
    ) -> ProgramResult {
        match Self::check(account) {
            Ok(_) => Ok(()),
            Err(_) => Self::init(account, payer, decimals, mint_authority, freeze_authority),
        }
    }
}

/// Helper for Token 2022 Token Account
pub struct Token2022;

impl Token2022 {
    /// Check if the account is a valid Token 2022 Token Account
    pub fn check(account: &AccountInfo) -> Result<(), ProgramError> {
        if !account.is_owned_by(&TOKEN_2022_PROGRAM_ID) {
            return Err(PinocchioError::InvalidOwner.into());
        }

        let data = account.try_borrow_data()?;

        if data.len().ne(&pinocchio_token::state::TokenAccount::LEN) {
            if data.len().le(&TOKEN_2022_ACCOUNT_DISCRIMINATOR_OFFSET) {
                return Err(PinocchioError::InvalidAccountData.into());
            }
            if data[TOKEN_2022_ACCOUNT_DISCRIMINATOR_OFFSET]
                .ne(&TOKEN_2022_TOKEN_ACCOUNT_DISCRIMINATOR)
            {
                return Err(PinocchioError::InvalidAccountData.into());
            }
        }

        Ok(())
    }

    /// Initialize a new Token 2022 Token Account
    pub fn init(
        account: &AccountInfo,
        mint: &AccountInfo,
        payer: &AccountInfo,
        owner: &[u8; 32],
    ) -> ProgramResult {
        // Get required lamports for rent
        let lamports = Rent::get()?.minimum_balance(pinocchio_token::state::TokenAccount::LEN);

        // Fund the account with the required lamports
        CreateAccount {
            from: payer,
            to: account,
            lamports,
            space: pinocchio_token::state::TokenAccount::LEN as u64,
            owner: &TOKEN_2022_PROGRAM_ID,
        }
        .invoke()?;

        InitializeAccount3 {
            account,
            mint,
            owner,
        }
        .invoke()
    }

    /// Initialize a new Token 2022 Token Account if it doesn't already exist
    pub fn init_if_needed(
        account: &AccountInfo,
        mint: &AccountInfo,
        payer: &AccountInfo,
        owner: &[u8; 32],
    ) -> ProgramResult {
        match Self::check(account) {
            Ok(_) => Ok(()),
            Err(_) => Self::init(account, mint, payer, owner),
        }
    }
}

// =============================================================================
// Token Interface Helpers (SPL Token + Token 2022)
// =============================================================================

/// Helper for Mint accounts that works with both SPL Token and Token 2022
pub struct MintInterface;

impl MintInterface {
    /// Check if the account is a valid Mint (either SPL Token or Token 2022)
    pub fn check(account: &AccountInfo) -> Result<(), ProgramError> {
        if !account.is_owned_by(&TOKEN_2022_PROGRAM_ID) {
            if !account.is_owned_by(&pinocchio_token::ID) {
                return Err(PinocchioError::InvalidOwner.into());
            } else {
                if account.data_len().ne(&pinocchio_token::state::Mint::LEN) {
                    return Err(PinocchioError::InvalidAccountData.into());
                }
            }
        } else {
            let data = account.try_borrow_data()?;

            if data.len().ne(&pinocchio_token::state::Mint::LEN) {
                if data.len().le(&TOKEN_2022_ACCOUNT_DISCRIMINATOR_OFFSET) {
                    return Err(PinocchioError::InvalidAccountData.into());
                }
                if data[TOKEN_2022_ACCOUNT_DISCRIMINATOR_OFFSET].ne(&TOKEN_2022_MINT_DISCRIMINATOR)
                {
                    return Err(PinocchioError::InvalidAccountData.into());
                }
            }
        }

        Ok(())
    }
}

/// Helper for Token accounts that works with both SPL Token and Token 2022
pub struct TokenInterface;

impl TokenInterface {
    /// Check if the account is a valid Token Account (either SPL Token or Token 2022)
    pub fn check(account: &AccountInfo) -> Result<(), ProgramError> {
        if !account.is_owned_by(&TOKEN_2022_PROGRAM_ID) {
            if !account.is_owned_by(&pinocchio_token::ID) {
                return Err(PinocchioError::InvalidOwner.into());
            } else {
                if account
                    .data_len()
                    .ne(&pinocchio_token::state::TokenAccount::LEN)
                {
                    return Err(PinocchioError::InvalidAccountData.into());
                }
            }
        } else {
            let data = account.try_borrow_data()?;

            if data.len().ne(&pinocchio_token::state::TokenAccount::LEN) {
                if data.len().le(&TOKEN_2022_ACCOUNT_DISCRIMINATOR_OFFSET) {
                    return Err(PinocchioError::InvalidAccountData.into());
                }
                if data[TOKEN_2022_ACCOUNT_DISCRIMINATOR_OFFSET]
                    .ne(&TOKEN_2022_TOKEN_ACCOUNT_DISCRIMINATOR)
                {
                    return Err(PinocchioError::InvalidAccountData.into());
                }
            }
        }

        Ok(())
    }
}

// =============================================================================
// Associated Token Account Helpers
// =============================================================================

/// Helper for Associated Token Accounts
pub struct AssociatedToken;

impl AssociatedToken {
    /// Check if the account is a valid Associated Token Account
    pub fn check(
        account: &AccountInfo,
        authority: &AccountInfo,
        mint: &AccountInfo,
        token_program: &AccountInfo,
    ) -> Result<(), ProgramError> {
        Token::check(account)?;

        if find_program_address(
            &[authority.key(), token_program.key(), mint.key()],
            &pinocchio_associated_token_account::ID,
        )
        .0
        .ne(account.key())
        {
            return Err(PinocchioError::InvalidAddress.into());
        }

        Ok(())
    }

    /// Initialize a new Associated Token Account
    pub fn init(
        account: &AccountInfo,
        mint: &AccountInfo,
        payer: &AccountInfo,
        owner: &AccountInfo,
        system_program: &AccountInfo,
        token_program: &AccountInfo,
    ) -> ProgramResult {
        Create {
            funding_account: payer,
            account,
            wallet: owner,
            mint,
            system_program,
            token_program,
        }
        .invoke()
    }

    /// Initialize a new Associated Token Account if it doesn't already exist
    pub fn init_if_needed(
        account: &AccountInfo,
        mint: &AccountInfo,
        payer: &AccountInfo,
        owner: &AccountInfo,
        system_program: &AccountInfo,
        token_program: &AccountInfo,
    ) -> ProgramResult {
        match Self::check(account, payer, mint, token_program) {
            Ok(_) => Ok(()),
            Err(_) => Self::init(account, mint, payer, owner, system_program, token_program),
        }
    }
}

// =============================================================================
// Program Account Helpers
// =============================================================================

/// Helper for Program-owned accounts
pub struct ProgramAccount;

impl ProgramAccount {
    /// Check if the account is a valid program-owned account
    pub fn check<const LEN: usize>(account: &AccountInfo) -> Result<(), ProgramError> {
        if !account.is_owned_by(&crate::ID) {
            return Err(PinocchioError::InvalidOwner.into());
        }

        if account.data_len() != LEN {
            return Err(PinocchioError::InvalidAccountData.into());
        }

        Ok(())
    }

    /// Initialize a new program-owned account with PDA seeds
    pub fn init<'a>(
        payer: &AccountInfo,
        account: &AccountInfo,
        seeds: &[Seed<'a>],
        space: usize,
    ) -> ProgramResult {
        // Get required lamports for rent
        let lamports = Rent::get()?.minimum_balance(space);

        // Create signer with seeds slice
        let signer = [Signer::from(seeds)];

        // Create the account
        CreateAccount {
            from: payer,
            to: account,
            lamports,
            space: space as u64,
            owner: &crate::ID,
        }
        .invoke_signed(&signer)?;

        Ok(())
    }

    /// Close a program-owned account and transfer lamports to destination
    pub fn close(account: &AccountInfo, destination: &AccountInfo) -> ProgramResult {
        {
            let mut data = account.try_borrow_mut_data()?;
            data[0] = 0xff;
        }

        *destination.try_borrow_mut_lamports()? += *account.try_borrow_lamports()?;
        account.realloc(1, true)?;
        account.close()
    }
}
