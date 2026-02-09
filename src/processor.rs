use pinocchio::{AccountView, Address, ProgramResult};

use crate::instructions::VaultInstruction;

pub struct Processor;

impl Processor {
    pub fn process(program_id: &Address, accounts: &[AccountView], data: &[u8]) -> ProgramResult {
        let instruction = VaultInstruction::unpack(data)?;

        instruction.process(program_id, accounts)
    }
}
