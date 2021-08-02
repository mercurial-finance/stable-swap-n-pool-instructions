pub mod instruction;
pub mod state;
pub mod error;
pub mod utils;

use solana_program::{entrypoint::ProgramResult, program_error::ProgramError, pubkey::Pubkey};

solana_program::declare_id!("MERLuDFBMmsHnsBPZw2sDQZHvXFMwp8EdjudcU2HKky");

/// Checks that the supplied program ID is the correct one for this program
pub fn check_program_account(program_id: &Pubkey) -> ProgramResult {
    if program_id != &id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

pub struct PoolParameter {}

impl PoolParameter {
    /// Maximum number of coins in a pool
    pub const MAX_N_COINS: usize = 4;
}
