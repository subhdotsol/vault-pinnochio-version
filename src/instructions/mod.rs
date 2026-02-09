pub mod deposit;
pub mod initialize;
pub mod withdraw;

use pinocchio::error::ProgramError;
use pinocchio::{AccountView, Address, ProgramResult};

pub enum VaultInstruction {
    /// Initialize a vault. Data: [bump: u8]
    Initialize { bump: u8 },
    /// Deposit SOL into the vault. Data: [amount: u64]
    Deposit { amount: u64 },
    /// Withdraw SOL from the vault. Data: [amount: u64, bump: u8]
    Withdraw { amount: u64, bump: u8 },
}

impl VaultInstruction {
    pub fn unpack(data: &[u8]) -> Result<Self, ProgramError> {
        if data.is_empty() {
            return Err(ProgramError::InvalidInstructionData);
        }

        Ok(match data[0] {
            0 => {
                if data.len() < 2 {
                    return Err(ProgramError::InvalidInstructionData);
                }
                Self::Initialize { bump: data[1] }
            }
            1 => {
                if data.len() < 9 {
                    return Err(ProgramError::InvalidInstructionData);
                }
                let amount = u64::from_le_bytes(data[1..9].try_into().unwrap());
                Self::Deposit { amount }
            }
            2 => {
                if data.len() < 10 {
                    return Err(ProgramError::InvalidInstructionData);
                }
                let amount = u64::from_le_bytes(data[1..9].try_into().unwrap());
                let bump = data[9];
                Self::Withdraw { amount, bump }
            }
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }

    pub fn process(&self, program_id: &Address, accounts: &[AccountView]) -> ProgramResult {
        match self {
            Self::Initialize { bump } => initialize::handler(program_id, accounts, *bump),
            Self::Deposit { amount } => deposit::handler(program_id, accounts, *amount),
            Self::Withdraw { amount, bump } => {
                withdraw::handler(program_id, accounts, *amount, *bump)
            }
        }
    }
}
