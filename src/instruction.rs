//! Instruction (De)serialization
//!
//! This module is responsible for
//! - converting incoming instruction data into a [SwapInstruction]
//! - converting a [SwapInstruction] into byte slices
//! - providing functions for downstream users to easily build [SwapInstruction]s

use solana_program::instruction::AccountMeta;
use solana_program::instruction::Instruction;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use std::convert::TryInto;
use std::mem::size_of;

use crate::check_program_account;
use crate::error::SwapError;
use crate::state::AdminSettings;
use crate::utils;
use crate::PoolParameter;

// Instructions for the stable swap.
#[repr(C)]
#[derive(Debug, PartialEq, Clone)]
pub enum SwapInstruction {
    /// Initializes the stable swap.
    ///
    /// Accounts expected:
    ///
    /// 0. `[writable]` New stable swap to be initialized.
    /// 1. `[]` The $authority derived from `create_program_address(&[stable swap account])`.
    /// 2. `[]` An array of token accounts, owned by $authority depending on N_COINS.
    /// 3. `[]` An array of token mints, mint corresponding to the token accounts above
    /// 4. `[]` The pool token mint, mint authority == $authority.
    /// 5. `[]` The admin token mint
    ///
    Initialize {
        /// The nonce for program address initialization
        nonce: u8,
        amplification_coefficient: u64,
        fee_numerator: u64,
        admin_fee_numerator: u64,
        n_coins: u8,
        admin_settings: AdminSettings,
    },
    /// Adds liquidity to the stable swap.
    ///
    /// Accounts expected:
    ///
    /// 0. `[writable]` The stable swap.
    /// 1. `[]` Token program id.
    /// 2. `[]` The $authority.
    /// 3. `[]` The user transfer authority
    /// 4. `[writable]` An array of token accounts, owned by $authority depending on N_COINS.
    /// 5. `[writable]` The pool token mint, owned by $authority.
    /// 6. `[writable]` An array of source token accounts, owned by the LP, depending on N_COINS.
    /// 7. `[writable, owned by Token Program, mint == pool_mint]` pool token account LP tokens get sent to.
    ///
    AddLiquidity {
        /// The deposit amounts depending on N_COINS
        deposit_amounts: Vec<u64>,
        /// The expected minimum mint amount by the LP
        min_mint_amount: u64,
    },
    /// Removes liquidity from the stable swap.
    ///
    /// Accounts expected:
    ///
    /// 0. `[]` The stable swap.
    /// 1. `[]` Token program id.
    /// 2. `[]` The $authority.
    /// 3. `[]` The user transfer authority
    /// 4. `[writable]` An array of token accounts, owned by $authority depending on N_COINS.
    /// 5. `[writable]` The pool token mint, owned by $authority.
    /// 6. `[writable]` An array of destination token accounts, owned by the LP.
    /// 7. `[writable]` LP token account, owned by the LP, can be burned by $authority.
    ///
    RemoveLiquidity {
        /// The unmint amount by the LP
        unmint_amount: u64,
        /// The minimum exected amounts to be received by the LP depending on N_COINS
        minimum_amounts: Vec<u64>,
    },
    /// Removes liquidity from the stable swap, one single token out.
    ///
    /// Accounts expected:
    ///
    /// 0. `[]` The stable swap.
    /// 1. `[]` Token program id.
    /// 2. `[]` The $authority.
    /// 3. `[]` The user transfer authority
    /// 4. `[writable]` An array of token accounts, owned by $authority depending on N_COINS.
    /// 5. `[writable]` The pool token mint, owned by $authority.
    /// 6. `[writable]` The destination token account, owned by the LP.
    /// 7. `[writable]` LP token account, owned by the LP, can be burned by $authority.
    ///
    RemoveLiquidityOneToken {
        /// The unmint amount by the LP
        unmint_amount: u64,
        /// The minimum exected amount to be received by the LP
        minimum_out_amount: u64,
    },
    /// Exchanges token[i] for token[y] from the stable swap.
    ///
    /// Accounts expected:
    ///
    /// 0. `[]` The stable swap.
    /// 1. `[]` Token program id.
    /// 2. `[]` The $authority.
    /// 3. `[]` The user transfer authority
    /// 4. `[writable]` The token accounts of the swap state, owned by $authority depending on N_COINS.
    /// 5. `[writable]` The source token account, owned by the LP, can be transferred by $authority.
    /// 6. `[writable]` The destination token account, owned by the LP.
    ///
    Exchange {
        in_amount: u64,
        minimum_out_amount: u64,
    },
    /// Get pool virtual price
    ///
    /// Accounts expected:
    ///
    /// Single Signer
    ///
    /// 0. `[]` Swap state account
    /// 1. `[]` Token program id.
    /// 2. `[]` An array of token accounts, owned by $authority depending on N_COINS.
    /// 3. `[]` The pool token mint, owned by $authority.
    GetVirtualPrice {},
}

impl SwapInstruction {
    /// Unpacks a byte buffer into a [SwapInstruction](enum.SwapInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (tag, rest) = Self::unpack_u8(input)?;

        Ok(match tag {
            0 => {
                let (nonce, rest) = Self::unpack_u8(rest)?;
                let (n_coins, rest) = Self::unpack_u8(rest)?;
                let (amplification_coefficient, rest) = Self::unpack_u64(rest)?;
                let (fee_numerator, rest) = Self::unpack_u64(rest)?;
                let (admin_fee_numerator, rest) = Self::unpack_u64(rest)?;
                let (swap_enabled, rest) = Self::unpack_u8(rest)?;
                let (add_liquidity_enabled, _) = Self::unpack_u8(rest)?;

                Self::Initialize {
                    nonce,
                    n_coins,
                    amplification_coefficient,
                    fee_numerator,
                    admin_fee_numerator,
                    admin_settings: AdminSettings {
                        swap_enabled: utils::u8_to_bool(swap_enabled)?,
                        add_liquidity_enabled: utils::u8_to_bool(add_liquidity_enabled)?,
                    },
                }
            }
            1 => {
                let mut deposit_amounts = Vec::with_capacity(PoolParameter::MAX_N_COINS);
                let (length, rest) = Self::unpack_u32(rest)?;
                for i in 0..length as usize {
                    let (_amount, rest) = rest.split_at(i * 8);
                    let (deposit_amount, _rest) = Self::unpack_u64(rest)?;
                    deposit_amounts.push(deposit_amount);
                }

                let (_amount, rest) = rest.split_at(length as usize * 8);
                let (min_mint_amount, _rest) = Self::unpack_u64(rest)?;
                Self::AddLiquidity {
                    deposit_amounts,
                    min_mint_amount,
                }
            }
            2 => {
                let (unmint_amount, rest) = Self::unpack_u64(rest)?;

                let mut minimum_amounts = Vec::with_capacity(PoolParameter::MAX_N_COINS);
                let (length, rest) = Self::unpack_u32(rest)?;
                for i in 0..length as usize {
                    let (_amount, rest) = rest.split_at(i * 8);
                    let (minimum_amount, _rest) = Self::unpack_u64(rest)?;
                    minimum_amounts.push(minimum_amount);
                }

                Self::RemoveLiquidity {
                    unmint_amount,
                    minimum_amounts,
                }
            }
            3 => {
                let (unmint_amount, rest) = Self::unpack_u64(rest)?;
                let (minimum_out_amount, _rest) = Self::unpack_u64(rest)?;

                Self::RemoveLiquidityOneToken {
                    unmint_amount,
                    minimum_out_amount,
                }
            }
            4 => {
                let (in_amount, rest) = Self::unpack_u64(rest)?;
                let (minimum_out_amount, _rest) = Self::unpack_u64(rest)?;

                Self::Exchange {
                    in_amount,
                    minimum_out_amount,
                }
            }
            5 => Self::GetVirtualPrice {},
            _ => return Err(ProgramError::InvalidAccountData.into()),
        })
    }

    /// Packs a [SwapInstruction](enum.SwapInstruction.html) into a byte buffer.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(size_of::<Self>());

        match self {
            Self::Initialize {
                nonce,
                n_coins,
                amplification_coefficient,
                fee_numerator,
                admin_fee_numerator,
                admin_settings,
            } => {
                buf.push(0);
                buf.push(*nonce);
                buf.push(*n_coins);
                buf.extend_from_slice(&u64::to_le_bytes(*amplification_coefficient));
                buf.extend_from_slice(&u64::to_le_bytes(*fee_numerator));
                buf.extend_from_slice(&u64::to_le_bytes(*admin_fee_numerator));
                buf.push(admin_settings.swap_enabled as u8);
                buf.push(admin_settings.add_liquidity_enabled as u8);
            }
            Self::AddLiquidity {
                deposit_amounts,
                min_mint_amount,
            } => {
                buf.push(1);

                // deposit amounts
                buf.extend_from_slice(&(deposit_amounts.len() as u32).to_le_bytes());
                for deposit_amount in deposit_amounts.iter() {
                    buf.extend_from_slice(&deposit_amount.to_le_bytes());
                }

                // min_mint_amount
                buf.extend_from_slice(&min_mint_amount.to_le_bytes());
            }
            Self::RemoveLiquidity {
                unmint_amount,
                minimum_amounts,
            } => {
                buf.push(2);

                // unmint_amount
                buf.extend_from_slice(&unmint_amount.to_le_bytes());

                // minimum amounts
                buf.extend_from_slice(&(minimum_amounts.len() as u32).to_le_bytes());
                for minimum_amount in minimum_amounts.iter() {
                    buf.extend_from_slice(&minimum_amount.to_le_bytes());
                }
            }
            Self::RemoveLiquidityOneToken {
                unmint_amount,
                minimum_out_amount,
            } => {
                buf.push(3);

                // unmint_amount
                buf.extend_from_slice(&unmint_amount.to_le_bytes());

                // minimum_out_amount
                buf.extend_from_slice(&minimum_out_amount.to_le_bytes());
            }
            Self::Exchange {
                in_amount,
                minimum_out_amount,
            } => {
                buf.push(4);

                // in_amount
                buf.extend_from_slice(&in_amount.to_le_bytes());

                // minimum_out_amount
                buf.extend_from_slice(&minimum_out_amount.to_le_bytes());
            }
            Self::GetVirtualPrice {} => buf.push(5),
        }
        buf
    }

    fn unpack_u8(input: &[u8]) -> Result<(u8, &[u8]), ProgramError> {
        let (&amount, rest) = input.split_first().ok_or(SwapError::InvalidInstruction)?;
        Ok((amount, rest))
    }

    fn unpack_u32(input: &[u8]) -> Result<(u32, &[u8]), ProgramError> {
        if input.len() >= 4 {
            let (amount, rest) = input.split_at(4);
            let amount = amount
                .get(..4)
                .and_then(|slice| slice.try_into().ok())
                .map(u32::from_le_bytes)
                .ok_or(SwapError::InvalidInstruction)?;
            Ok((amount, rest))
        } else {
            Err(SwapError::InvalidInstruction.into())
        }
    }

    fn unpack_u64(input: &[u8]) -> Result<(u64, &[u8]), ProgramError> {
        if input.len() >= 8 {
            let (amount, rest) = input.split_at(8);
            let amount = amount
                .get(..8)
                .and_then(|slice| slice.try_into().ok())
                .map(u64::from_le_bytes)
                .ok_or(SwapError::InvalidInstruction)?;
            Ok((amount, rest))
        } else {
            Err(SwapError::InvalidInstruction.into())
        }
    }
}

/// Creates a [SwapInstruction::Initialize] instruction
pub fn initialize(
    program_id: &Pubkey,
    swap_account_address: &Pubkey,
    pool_authority_address: &Pubkey,
    swap_token_accounts_addresses: Vec<&Pubkey>,
    swap_token_mint_addresses: Vec<&Pubkey>,
    pool_token_mint_address: &Pubkey,
    admin_token_mint_address: &Pubkey,
    nonce: u8,
    n_coins: u8,
    amplification_coefficient: u64,
    fee_numerator: u64,
    admin_fee_numerator: u64,
    admin_settings: AdminSettings,
) -> Result<Instruction, ProgramError> {
    check_program_account(program_id)?; // TODO: taken from token program but can we remove this? if it only accepts 1 program_id why not just hardcode it?

    let mut accounts = Vec::with_capacity(3 + PoolParameter::MAX_N_COINS);
    accounts.push(AccountMeta::new(*swap_account_address, false));
    accounts.push(AccountMeta::new_readonly(*pool_authority_address, false));
    for token_account in swap_token_accounts_addresses {
        accounts.push(AccountMeta::new_readonly(*token_account, false));
    }
    for token_mint_address in swap_token_mint_addresses {
        accounts.push(AccountMeta::new_readonly(*token_mint_address, false));
    }
    accounts.push(AccountMeta::new_readonly(*pool_token_mint_address, false));
    accounts.push(AccountMeta::new_readonly(*admin_token_mint_address, false));

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data: SwapInstruction::Initialize {
            nonce,
            n_coins,
            amplification_coefficient,
            fee_numerator,
            admin_fee_numerator,
            admin_settings,
        }
        .pack(),
    })
}

/// Creates a [SwapInstruction::AddLiquidity] instruction
pub fn add_liquidity(
    program_id: &Pubkey,
    swap_account_address: &Pubkey,
    token_program_address: &Pubkey,
    pool_authority_address: &Pubkey,
    user_transfer_authority_address: &Pubkey,
    swap_token_addresses: Vec<&Pubkey>,
    pool_token_mint_address: &Pubkey,
    source_token_addresses: Vec<&Pubkey>,
    lp_token_account_address: &Pubkey,
    deposit_amounts: Vec<u64>,
    min_mint_amount: u64,
) -> Result<Instruction, ProgramError> {
    check_program_account(program_id)?; // TODO: taken from token program but can we remove this? if it only accepts 1 program_id why not just hardcode it?

    let mut accounts = Vec::with_capacity(2 * PoolParameter::MAX_N_COINS + 5);
    accounts.push(AccountMeta::new_readonly(*swap_account_address, false));
    accounts.push(AccountMeta::new_readonly(*token_program_address, false));
    accounts.push(AccountMeta::new_readonly(*pool_authority_address, false));
    accounts.push(AccountMeta::new_readonly(
        *user_transfer_authority_address,
        true,
    ));
    for token_account_address in swap_token_addresses {
        accounts.push(AccountMeta::new(*token_account_address, false));
    }
    accounts.push(AccountMeta::new(*pool_token_mint_address, false));
    for token_account_address in source_token_addresses {
        accounts.push(AccountMeta::new(*token_account_address, false));
    }
    accounts.push(AccountMeta::new(*lp_token_account_address, false));

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data: SwapInstruction::AddLiquidity {
            deposit_amounts,
            min_mint_amount,
        }
        .pack(),
    })
}

/// Creates a [SwapInstruction::RemoveLiquidity] instruction
pub fn remove_liquidity(
    program_id: &Pubkey,
    swap_account_address: &Pubkey,
    token_program_address: &Pubkey,
    pool_authority_address: &Pubkey,
    user_transfer_authority_address: &Pubkey,
    swap_token_accounts_addresses: Vec<&Pubkey>,
    pool_mint_address: &Pubkey,
    user_destination_token_account_addresses: Vec<&Pubkey>,
    lp_token_account_address: &Pubkey,
    unmint_amount: u64,
    minimum_amounts: Vec<u64>,
) -> Result<Instruction, ProgramError> {
    check_program_account(program_id)?;

    let mut accounts = Vec::with_capacity(PoolParameter::MAX_N_COINS + 5);
    accounts.push(AccountMeta::new_readonly(*swap_account_address, false));
    accounts.push(AccountMeta::new_readonly(*token_program_address, false));
    accounts.push(AccountMeta::new_readonly(*pool_authority_address, false));
    accounts.push(AccountMeta::new_readonly(
        *user_transfer_authority_address,
        true,
    ));
    for token_account_address in swap_token_accounts_addresses {
        accounts.push(AccountMeta::new(*token_account_address, false));
    }
    accounts.push(AccountMeta::new(*pool_mint_address, false));
    for user_destination_token_account_address in user_destination_token_account_addresses {
        accounts.push(AccountMeta::new(
            *user_destination_token_account_address,
            false,
        ));
    }
    accounts.push(AccountMeta::new(*lp_token_account_address, false));

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data: SwapInstruction::RemoveLiquidity {
            unmint_amount,
            minimum_amounts,
        }
        .pack(),
    })
}

/// Creates a [SwapInstruction::RemoveLiquidityOneToken] instruction
pub fn remove_liquidity_one_token(
    program_id: &Pubkey,
    swap_account_address: &Pubkey,
    token_program_address: &Pubkey,
    pool_authority_address: &Pubkey,
    user_transfer_authority_address: &Pubkey,
    swap_token_accounts_addresses: Vec<&Pubkey>,
    pool_mint_address: &Pubkey,
    user_destination_token_account_address: &Pubkey,
    lp_token_account_address: &Pubkey,
    unmint_amount: u64,
    minimum_out_amount: u64,
) -> Result<Instruction, ProgramError> {
    check_program_account(program_id)?;

    let mut accounts = Vec::with_capacity(PoolParameter::MAX_N_COINS + 5);
    accounts.push(AccountMeta::new_readonly(*swap_account_address, false));
    accounts.push(AccountMeta::new_readonly(*token_program_address, false));
    accounts.push(AccountMeta::new_readonly(*pool_authority_address, false));
    accounts.push(AccountMeta::new_readonly(
        *user_transfer_authority_address,
        true,
    ));
    for token_account_address in swap_token_accounts_addresses {
        accounts.push(AccountMeta::new(*token_account_address, false));
    }
    accounts.push(AccountMeta::new(*pool_mint_address, false));
    accounts.push(AccountMeta::new(
        *user_destination_token_account_address,
        false,
    ));
    accounts.push(AccountMeta::new(*lp_token_account_address, false));

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data: SwapInstruction::RemoveLiquidityOneToken {
            unmint_amount,
            minimum_out_amount,
        }
        .pack(),
    })
}

/// Creates a [SwapInstruction::Exchange] instruction
pub fn exchange(
    program_id: &Pubkey,
    swap_account_address: &Pubkey,
    token_program_address: &Pubkey,
    pool_authority_address: &Pubkey,
    user_transfer_authority_address: &Pubkey,
    swap_token_accounts_addresses: Vec<&Pubkey>,
    source_token_account_address: &Pubkey,
    destination_token_account_address: &Pubkey,
    in_amount: u64,
    minimum_out_amount: u64,
) -> Result<Instruction, ProgramError> {
    check_program_account(program_id)?;

    let mut accounts = Vec::with_capacity(PoolParameter::MAX_N_COINS + 5);
    accounts.push(AccountMeta::new_readonly(*swap_account_address, false));
    accounts.push(AccountMeta::new_readonly(*token_program_address, false));
    accounts.push(AccountMeta::new_readonly(*pool_authority_address, false));
    accounts.push(AccountMeta::new_readonly(
        *user_transfer_authority_address,
        true,
    ));
    for token_account_address in swap_token_accounts_addresses {
        accounts.push(AccountMeta::new(*token_account_address, false));
    }
    accounts.push(AccountMeta::new(*source_token_account_address, false));
    accounts.push(AccountMeta::new(*destination_token_account_address, false));

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data: SwapInstruction::Exchange {
            in_amount,
            minimum_out_amount,
        }
        .pack(),
    })
}