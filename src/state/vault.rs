use pinocchio::{AccountView, Address};

/// Vault account discriminator
pub const VAULT_DISCRIMINATOR: [u8; 8] = [0x56, 0x61, 0x75, 0x6c, 0x74, 0x21, 0x21, 0x21]; // "Vault!!!"

/// Vault state account layout:
/// - [0..8]   discriminator (8 bytes)
/// - [8..40]  owner (32 bytes)
/// - [40..48] amount (8 bytes, u64 LE)
pub struct Vault(*const u8);

impl Vault {
    pub const LEN: usize = 8 + 32 + 8; // 48 bytes

    pub const DISCRIMINATOR_OFFSET: usize = 0;
    pub const OWNER_OFFSET: usize = 8;
    pub const AMOUNT_OFFSET: usize = 40;

    /// Create a Vault from an AccountView reference
    ///
    /// # Safety
    /// The caller must ensure the account data is valid and has the correct length.
    pub fn from_account_unchecked(account: &AccountView) -> Self {
        unsafe { Self(account.borrow_unchecked().as_ptr()) }
    }

    /// Create a Vault from an AccountView, checking discriminator and length
    pub fn from_account(account: &AccountView) -> Self {
        assert_eq!(account.data_len(), Self::LEN, "Invalid vault data length");

        let vault = Self::from_account_unchecked(account);

        assert_eq!(
            vault.discriminator(),
            VAULT_DISCRIMINATOR,
            "Invalid vault discriminator"
        );

        vault
    }

    /// Get the discriminator
    pub fn discriminator(&self) -> [u8; 8] {
        unsafe { *(self.0.add(Self::DISCRIMINATOR_OFFSET) as *const [u8; 8]) }
    }

    /// Get the owner pubkey
    pub fn owner(&self) -> &Address {
        unsafe { &*(self.0.add(Self::OWNER_OFFSET) as *const Address) }
    }

    /// Get the amount (u64)
    pub fn amount(&self) -> u64 {
        unsafe { u64::from_le_bytes(*(self.0.add(Self::AMOUNT_OFFSET) as *const [u8; 8])) }
    }
}
