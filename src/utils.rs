use solana_program::{
    program_error::ProgramError,
};

/// Converts u8 to bool if u8 == 0 or u8 == 1
pub fn u8_to_bool(num: u8) -> Result<bool, ProgramError> {
    match num {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(ProgramError::InvalidAccountData),
    }
}