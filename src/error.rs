use num_derive::FromPrimitive;
use solana_program::{
    decode_error::DecodeError,
    program_error::ProgramError,
};
use thiserror::Error;

#[derive(Error, Debug, Copy, Clone, FromPrimitive, Eq, PartialEq)]
pub enum SwapError {
    // 0
    /// Invalid instruction
    #[error("Invalid Instruction")]
    InvalidInstruction, // TODO: remove this, there already exists a ProgramError for this error
    /// Not Rent Exempt
    #[error("Not Rent Exempt")]
    NotRentExempt,
    /// Invalid Program Address
    #[error("Invalid Program Address")]
    InvalidProgramAddress,
    /// Incorrect Token Program Id
    #[error("Incorrect Token Program Id")]
    IncorrectTokenProgramId,
    /// Expected Account
    #[error("Expected Account")]
    ExpectedAccount,

    // 5
    /// Expected Mint
    #[error("Expected Mint")]
    ExpectedMint,
    /// Invalid Owner
    #[error("Invalid Owner")]
    InvalidOwner,
    /// Invalid Token Match
    #[error("Invalid Token Match")]
    InvalidTokenMatch,
    /// Invalid Conversion
    #[error("Invalid Conversion")]
    InvalidConversion,
    /// Invalid Initial Deposit
    #[error("Invalid Initial Deposit")]
    InvalidInitialDeposit,

    // 10
    /// Invalid Calculation
    #[error("Invalid Calculation")]
    InvalidCalculation,
    /// ExceededSlippage
    #[error("ExceededSlippage")]
    ExceededSlippage,
    /// Invalid Exchange Account
    #[error("Invalid Exchange Account")]
    InvalidExchangeAccount,
    /// Invalid Mint
    #[error("Invalid Mint")]
    InvalidMint,
    /// Swap Already Initialized
    #[error("Swap Already Initialized")]
    SwapAlreadyInitialized,

    // 15
    /// Token Account Frozen
    #[error("Token Account Frozen")]
    TokenAccountFrozen,
    /// Delegated Token Account
    #[error("Delegated Token Account")]
    DelegatedTokenAccount,
    /// Mint Freeze Authority Set
    #[error("Mint Freeze Authority Set")]
    MintFreezeAuthoritySet,
    /// Close Authority Set
    #[error("Close Authority Set")]
    CloseAuthoritySet,
    /// Repeated Mint
    #[error("Repeated Mint")]
    RepeatedMint,

    // 20
    /// Token Account Not Empty
    #[error("Token Account Not Empty")]
    TokenAccountNotEmpty,
    /// Pool Token Supply Not Empty
    #[error("Pool Token Supply Not Empty")]
    PoolTokenSupplyNotEmpty,
    /// Invalid Token Account
    #[error("Invalid Token Account")]
    InvalidTokenAccount,
    /// Invalid Admin Mint Decimals
    #[error("Invalid Admin Mint Decimals")]
    InvalidAdminMintDecimals,
    /// Swap Disabled
    #[error("Swap Disabled")]
    SwapDisabled,

    // 25
    /// Add Liquidity Disabled
    #[error("Add Liquidity Disabled")]
    AddLiquidityDisabled,
    /// Admin Only
    #[error("Admin Only")]
    NoAdminTokens,
    /// Invalid Admin Delegate
    #[error("Invalid Admin Delegate")]
    InvalidAdminDelegate,
    /// Admin Token Account Frozen
    #[error("Admin Token Account Frozen")]
    AdminTokenAccountFrozen,
    /// Pool Token Decimals Invalid
    #[error("Pool Token Decimals Invalid")]
    PoolTokenDecimalsInvalid,
}

impl From<SwapError> for ProgramError {
    fn from(e: SwapError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for SwapError {
    fn type_of() -> &'static str {
        "Swap Error"
    }
}
