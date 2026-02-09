use pinocchio::{AccountView, Address, ProgramResult, entrypoint};

use crate::processor::Processor;

entrypoint!(process_instruction);

fn process_instruction(
    program_id: &Address,
    accounts: &[AccountView],
    data: &[u8],
) -> ProgramResult {
    Processor::process(program_id, accounts, data)
}
