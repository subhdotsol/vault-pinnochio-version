pub mod deposit;
pub mod withdraw;

use pinocchio::error::ProgramError;
use pinocchio::{AccountView, Address, ProgramResult};

pub enum VaultInstruction {
    Deposit { amount: u64 },
    Withdraw { amount: u64 },
}

impl VaultInstruction {
    pub fn unpack(data: &[u8]) -> Result<Self, ProgramError> {
        Ok(match data[0] {
            0 => {
                let amount = u64::from_le_bytes(data[1..9].try_into().unwrap());
                Self::Deposit { amount }
            }
            1 => {
                let amount = u64::from_le_bytes(data[1..9].try_into().unwrap());
                Self::Withdraw { amount }
            }
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }

    pub fn process(&self, program_id: &Address, accounts: &[AccountView]) -> ProgramResult {
        match self {
            Self::Deposit { amount } => deposit::handler(program_id, accounts, *amount),
            Self::Withdraw { amount } => withdraw::handler(program_id, accounts, *amount),
        }
    }
}
