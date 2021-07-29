use crate::PoolParameter;
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct AdminSettings {
    pub swap_enabled: bool,
    pub add_liquidity_enabled: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SwapVersion {
    /// Latest version
    SwapV2(SwapV2),
}

/// Current used state, previous state is only usable for migration
pub type SwapState = SwapV2;

#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SwapV2 {
    /// Initialized state.
    pub is_initialized: bool,

    /// Nonce used in program address.
    /// The program address is created deterministically with the nonce,
    /// swap program id, and swap account pubkey. This program address has
    /// authority over the swap's token accounts, and pool token mint.
    pub nonce: u8,
    /// Amplification coefficient for curve computations
    pub amplification_coefficient: u64,
    /// LP fee numerator for LP fee curve computations
    pub fee_numerator: u64,
    /// Admin fee numerator, not implemented
    pub admin_fee_numerator: u64,
    pub precision_factor: u64,
    pub precision_multipliers: Vec<u64>,
    pub token_account_addresses: Vec<Pubkey>,
    pub pool_mint_address: Pubkey,
    pub admin_token_mint_address: Pubkey,
    pub admin_settings: AdminSettings,
}

impl SwapVersion {
    /// Size of the latest version of the SwapState
    pub const LATEST_LEN: usize = 1 + SwapV2::LEN; // add one for the version enum

    /// Pack a swap into a byte array, based on its version
    pub fn pack(src: Self, dst: &mut [u8]) -> Result<(), ProgramError> {
        match src {
            Self::SwapV2(swap_info) => {
                dst[0] = 2;
                SwapV2::pack(swap_info, &mut dst[1..])
            }
        }
    }

    /// Unpack the swap account based on its version, returning the result as a
    /// SwapState trait object
    pub fn unpack(input: &[u8]) -> Result<SwapVersion, ProgramError> {
        let (&version, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidAccountData)?;
        match version {
            2 => Ok(Self::SwapV2(SwapV2::unpack(rest)?)),
            _ => Err(ProgramError::UninitializedAccount),
        }
    }

    /// Special check to be done before any instruction processing, works for
    /// all versions
    pub fn is_initialized(input: &[u8]) -> bool {
        match Self::unpack(input) {
            Ok(swap) => match swap {
                Self::SwapV2(swapv2) => swapv2.is_initialized,
            },
            Err(_) => false,
        }
    }
}

impl Sealed for SwapV2 {}

impl IsInitialized for SwapV2 {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

// Please note how this is very similar to SwapV1, when V3 is introduced, we can delete V1 and migrate from V2 to V3
impl Pack for SwapV2 {
    const LEN: usize = 1
        + 1
        + 8
        + 8
        + 8
        + 4
        + 8
        + PoolParameter::MAX_N_COINS * 8
        + PoolParameter::MAX_N_COINS * 32
        + 32
        + 32
        + 2;

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, SwapV2::LEN];
        let (
            is_initialized,
            nonce,
            amplification_coefficient,
            fee_numerator,
            admin_fee_numerator,
            tokens_len,
            precision_factor,
            multipliers,
            tokens,
            pool_mint,
            admin_token_mint,
            admin_settings,
        ) = array_refs![
            src,
            1,
            1,
            8,
            8,
            8,
            4,
            8,
            PoolParameter::MAX_N_COINS * 8,
            PoolParameter::MAX_N_COINS * 32,
            32,
            32,
            2
        ];

        let is_initialized = match is_initialized {
            [0] => false,
            [1] => true,
            _ => return Err(ProgramError::InvalidAccountData),
        };

        let mut precision_multipliers = Vec::with_capacity(PoolParameter::MAX_N_COINS);
        for i in 0..u32::from_le_bytes(*tokens_len) as usize {
            let multiplier = array_ref!(multipliers, i * 8, 8);
            precision_multipliers.push(u64::from_le_bytes(*multiplier));
        }

        let mut token_account_addresses = Vec::with_capacity(PoolParameter::MAX_N_COINS);
        for i in 0..u32::from_le_bytes(*tokens_len) as usize {
            let token = array_ref!(tokens, i * 32, 32);
            token_account_addresses.push(Pubkey::new_from_array(*token));
        }

        Ok(SwapV2 {
            is_initialized,
            nonce: nonce[0],
            amplification_coefficient: u64::from_le_bytes(*amplification_coefficient),
            fee_numerator: u64::from_le_bytes(*fee_numerator),
            admin_fee_numerator: u64::from_le_bytes(*admin_fee_numerator),
            precision_factor: u64::from_le_bytes(*precision_factor),
            precision_multipliers,
            token_account_addresses,
            pool_mint_address: Pubkey::new_from_array(*pool_mint),
            admin_token_mint_address: Pubkey::new_from_array(*admin_token_mint),
            admin_settings: AdminSettings {
                swap_enabled: match admin_settings[0] {
                    0 => false,
                    1 => true,
                    _ => return Err(ProgramError::InvalidAccountData),
                },
                add_liquidity_enabled: match admin_settings[1] {
                    0 => false,
                    1 => true,
                    _ => return Err(ProgramError::InvalidAccountData),
                },
            },
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, SwapV2::LEN];
        let (
            is_initialized_dst,
            nonce_dst,
            amplification_coefficient_dst,
            fee_numerator_dst,
            admin_fee_numerator_dst,
            token_account_addresses_len_dst,
            precision_factor_dst,
            precision_multipliers_dst,
            token_account_addresses_dst,
            pool_mint_address_dst,
            admin_token_mint_address_dst,
            admin_settings_dst,
        ) = mut_array_refs![
            dst,
            1,
            1,
            8,
            8,
            8,
            4,
            8,
            PoolParameter::MAX_N_COINS * 8,
            PoolParameter::MAX_N_COINS * 32,
            32,
            32,
            2
        ];

        let SwapV2 {
            is_initialized,
            nonce,
            amplification_coefficient,
            fee_numerator,
            admin_fee_numerator,
            precision_factor,
            precision_multipliers,
            token_account_addresses,
            pool_mint_address,
            admin_token_mint_address,
            admin_settings,
        } = self;

        is_initialized_dst[0] = *is_initialized as u8;
        nonce_dst[0] = *nonce;
        amplification_coefficient_dst
            .copy_from_slice(u64::to_le_bytes(*amplification_coefficient).as_ref());
        fee_numerator_dst.copy_from_slice(u64::to_le_bytes(*fee_numerator).as_ref());
        admin_fee_numerator_dst.copy_from_slice(u64::to_le_bytes(*admin_fee_numerator).as_ref());

        token_account_addresses_len_dst
            .copy_from_slice(u32::to_le_bytes(token_account_addresses.len() as u32).as_ref());

        precision_factor_dst.copy_from_slice(u64::to_le_bytes(*precision_factor).as_ref());
        for i in 0..precision_multipliers.len() {
            let multiplier_dst = array_mut_ref![precision_multipliers_dst, i * 8, 8];
            multiplier_dst.copy_from_slice(u64::to_le_bytes(precision_multipliers[i]).as_ref());
        }

        for i in 0..token_account_addresses.len() {
            let token_address_dst = array_mut_ref![token_account_addresses_dst, i * 32, 32];
            token_address_dst.copy_from_slice(token_account_addresses[i].as_ref());
        }

        pool_mint_address_dst.copy_from_slice(pool_mint_address.as_ref());
        admin_token_mint_address_dst.copy_from_slice(admin_token_mint_address.as_ref());
        admin_settings_dst[0] = admin_settings.swap_enabled as u8;
        admin_settings_dst[1] = admin_settings.add_liquidity_enabled as u8;
    }
}