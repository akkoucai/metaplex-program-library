use crate::{
    error::MetadataError,
    escrow::pda::find_escrow_seeds,
    instruction::MetadataInstruction,
    state::{
        EscrowAuthority, Key, Metadata, TokenMetadataAccount, TokenOwnedEscrow, TokenStandard,
    },
    utils::{
        assert_derivation, assert_initialized, assert_owned_by, assert_signer,
        check_token_standard, create_or_allocate_account_raw,
    },
};
use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    program_memory::sol_memcpy,
    pubkey::Pubkey,
};

pub fn create_escrow_account(
    program_id: Pubkey,
    escrow_account: Pubkey,
    metadata_account: Pubkey,
    mint_account: Pubkey,
    token_account: Pubkey,
    edition_account: Pubkey,
    payer_account: Pubkey,
    authority: Option<Pubkey>,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new(escrow_account, false),
        AccountMeta::new_readonly(metadata_account, false),
        AccountMeta::new_readonly(mint_account, false),
        AccountMeta::new_readonly(token_account, false),
        AccountMeta::new_readonly(edition_account, false),
        AccountMeta::new(payer_account, true),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    if let Some(authority) = authority {
        accounts.push(AccountMeta::new_readonly(authority, true));
    }

    let data = MetadataInstruction::CreateEscrowAccount
        .try_to_vec()
        .unwrap();

    Instruction {
        program_id,
        accounts,
        data,
    }
}

pub fn process_create_escrow_account(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let escrow_account_info = next_account_info(account_info_iter)?;
    let metadata_account_info = next_account_info(account_info_iter)?;
    let mint_account_info = next_account_info(account_info_iter)?;
    let token_account_info = next_account_info(account_info_iter)?;
    let edition_account_info = next_account_info(account_info_iter)?;
    let payer_account_info = next_account_info(account_info_iter)?;
    let system_account_info = next_account_info(account_info_iter)?;

    let is_using_authority = account_info_iter.len() == 1;

    let maybe_authority_info: Option<&AccountInfo> = if is_using_authority {
        Some(next_account_info(account_info_iter)?)
    } else {
        None
    };

    assert_owned_by(metadata_account_info, program_id)?;
    assert_owned_by(mint_account_info, &spl_token::id())?;
    assert_owned_by(token_account_info, &spl_token::id())?;
    assert_signer(payer_account_info)?;

    let metadata: Metadata = Metadata::from_account_info(metadata_account_info)?;

    // Mint account passed in must be the mint of the metadata account passed in.
    if &metadata.mint != mint_account_info.key {
        return Err(MetadataError::MintMismatch.into());
    }

    // Only non-fungible tokens (i.e. unique) can have escrow accounts.
    if check_token_standard(mint_account_info, Some(edition_account_info))?
        != TokenStandard::NonFungible
    {
        return Err(MetadataError::MustBeNonFungible.into());
    };

    let creator = maybe_authority_info.unwrap_or(payer_account_info);

    let token_account: spl_token::state::Account = assert_initialized(token_account_info)?;

    if token_account.mint != *mint_account_info.key {
        return Err(MetadataError::MintMismatch.into());
    }

    if token_account.amount < 1 {
        return Err(MetadataError::NotEnoughTokens.into());
    }

    if token_account.mint != metadata.mint {
        return Err(MetadataError::MintMismatch.into());
    }

    let creator_type = if token_account.owner == *creator.key {
        EscrowAuthority::TokenOwner
    } else {
        EscrowAuthority::Creator(*creator.key)
    };

    // Derive the seeds for PDA signing.
    let escrow_seeds = find_escrow_seeds(mint_account_info.key, &creator_type);

    let bump_seed = &[assert_derivation(
        &crate::id(),
        escrow_account_info,
        &escrow_seeds,
    )?];

    let escrow_authority_seeds = [escrow_seeds, vec![bump_seed]].concat();

    // Initialize a default (empty) escrow structure.
    let toe = TokenOwnedEscrow {
        key: Key::TokenOwnedEscrow,
        base_token: *mint_account_info.key,
        authority: creator_type,
        bump: bump_seed[0],
    };

    let serialized_data = toe
        .try_to_vec()
        .map_err(|_| MetadataError::BorshSerializationError)?;

    // Create the account.
    create_or_allocate_account_raw(
        *program_id,
        escrow_account_info,
        system_account_info,
        payer_account_info,
        serialized_data.len(),
        &escrow_authority_seeds,
    )?;

    sol_memcpy(
        &mut **escrow_account_info.try_borrow_mut_data().unwrap(),
        &serialized_data,
        serialized_data.len(),
    );

    Ok(())
}
