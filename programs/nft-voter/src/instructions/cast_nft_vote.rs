use crate::error::NftVoterError;
use crate::tools::governance::{add_voter_weight, DedSplGovernanceProgram};
use crate::{id, state::*};
use anchor_lang::prelude::*;
use anchor_lang::Accounts;
use itertools::Itertools;
use solana_program::sysvar;
use solana_program::sysvar::instructions::get_instruction_relative;

use spl_governance_tools::account::create_and_serialize_account_signed;

/// Casts NFT vote. The NFTs used for voting are tracked using NftVoteRecord accounts
/// This instruction updates VoterWeightRecord which is valid for the current Slot and the target Proposal only
/// and hance the instruction has to be executed inside the same transaction as spl-gov.CastVote
///
/// CastNftVote is accumulative and can be invoked using several transactions if voter owns more than 5 NFTs to calculate total voter_weight
/// In this scenario only the last CastNftVote should be bundled  with spl-gov.CastVote in the same transaction
///
/// CastNftVote instruction and NftVoteRecord are not directional. They don't record vote choice (ex Yes/No)
/// VoteChoice is recorded by spl-gov in VoteRecord and this CastNftVote only tracks voting NFTs
///
#[derive(Accounts)]
pub struct CastNftVote<'info> {
    /// The NFT voting registrar
    pub registrar: Account<'info, Registrar>,

    #[account(
        mut,
        constraint = voter_weight_record.realm == registrar.realm
        @ NftVoterError::InvalidVoterWeightRecordRealm,

        constraint = voter_weight_record.governing_token_mint == registrar.governing_token_mint
        @ NftVoterError::InvalidVoterWeightRecordMint,
    )]
    pub voter_weight_record: Account<'info, VoterWeightRecord>,

    /// The token owner who casts the vote
    #[account(
        address = voter_weight_record.governing_token_owner @ NftVoterError::InvalidTokenOwnerForVoterWeightRecord
    )]
    pub governing_token_owner: Signer<'info>,

    /// The account which pays for the transaction
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK
    #[account(
        owner = governance_program.key()
    )]
    pub proposal: AccountInfo<'info>,

    // // CHECK
    // #[account(
    //     owner = governance_program.key()
    // )]
    // pub proposal_transaction: AccountInfo<'info>,
    /// CHECK: Accounts checked in instruction
    #[account(address = sysvar::instructions::id())]
    instruction_sysvar_account: AccountInfo<'info>,
    pub governance_program: Program<'info, DedSplGovernanceProgram>,
    pub system_program: Program<'info, System>,
}

/// Casts vote with the NFT
pub fn cast_nft_vote<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, CastNftVote<'info>>,
) -> Result<()> {
    let registrar = &ctx.accounts.registrar;
    let governing_token_owner = &ctx.accounts.governing_token_owner.key();
    let mut voter_weight = 0u64;

    // Ensure all voting nfts in the batch are unique
    let mut unique_nft_mints = vec![];

    let proposal_transaction_info = if ctx.remaining_accounts.len() % 3 != 0 {
        ctx.remaining_accounts.last()
    } else {
        None
    };

    let rent = Rent::get()?;

    let cast_vote_spl_ix_result =
        get_instruction_relative(1, &ctx.accounts.instruction_sysvar_account);
    let cast_vote_spl_ix = if let Ok(ix) = cast_vote_spl_ix_result {
        //Check there are no more instructions after cast vote
        get_instruction_relative(2, &ctx.accounts.instruction_sysvar_account).unwrap_err();
        require_keys_eq!(ix.program_id, DedSplGovernanceProgram::id());
        Some(ix)
    } else {
        None
    };

    for (nft_info, nft_metadata_info, nft_vote_record_info) in
        ctx.remaining_accounts.iter().tuples()
    {
        let (nft_vote_weight, nft_mint) = resolve_nft_vote_weight_and_mint(
            registrar,
            governing_token_owner,
            nft_info,
            nft_metadata_info,
            &mut unique_nft_mints,
        )?;

        voter_weight = voter_weight.checked_add(nft_vote_weight as u64).unwrap();

        // Create NFT vote record to ensure the same NFT hasn't been already used for voting
        // Note: The correct PDA of the NftVoteRecord is validated in create_and_serialize_account_signed
        // It ensures the NftVoteRecord is for ('nft-vote-record',proposal,nft_mint) seeds
        require!(
            nft_vote_record_info.data_is_empty(),
            NftVoterError::NftAlreadyVoted
        );

        // Note: proposal.governing_token_mint must match voter_weight_record.governing_token_mint
        // We don't verify it here because spl-gov does the check in cast_vote
        // and it would reject voter_weight_record if governing_token_mint doesn't match

        // Note: Once the NFT plugin is enabled the governing_token_mint is used only as identity
        // for the voting population and the tokens of that mint are no longer used
        let nft_vote_record = NftVoteRecord {
            account_discriminator: NftVoteRecord::ACCOUNT_DISCRIMINATOR,
            proposal: ctx.accounts.proposal.key(),
            nft_mint,
            governing_token_owner: *governing_token_owner,
            reserved: [0; 8],
        };

        // Anchor doesn't natively support dynamic account creation using remaining_accounts
        // and we have to take it on the manual drive
        create_and_serialize_account_signed(
            &ctx.accounts.payer.to_account_info(),
            nft_vote_record_info,
            &nft_vote_record,
            &get_nft_vote_record_seeds(&ctx.accounts.proposal.key(), &nft_mint),
            &id(),
            &ctx.accounts.system_program.to_account_info(),
            &rent,
        )?;
    }

    let voter_weight_record = &mut ctx.accounts.voter_weight_record;

    if voter_weight_record.weight_action_target == Some(ctx.accounts.proposal.key())
        && voter_weight_record.weight_action == Some(VoterWeightAction::CastVote)
    {
        // If cast_nft_vote is called for the same proposal then we keep accumulating the weight
        // this way cast_nft_vote can be called multiple times in different transactions to allow voting with any number of NFTs
        add_voter_weight(
            &ctx.accounts.proposal,
            &ctx.accounts.governance_program.key(),
            proposal_transaction_info,
            cast_vote_spl_ix,
            voter_weight,
            voter_weight_record,
        )?;
    } else {
        voter_weight_record.voter_weight = 0;
        add_voter_weight(
            &ctx.accounts.proposal,
            &ctx.accounts.governance_program.key(),
            proposal_transaction_info,
            cast_vote_spl_ix,
            voter_weight,
            voter_weight_record,
        )?;
    }

    // The record is only valid as of the current slot
    voter_weight_record.voter_weight_expiry = Some(Clock::get()?.slot);

    // The record is only valid for casting vote on the given Proposal
    voter_weight_record.weight_action = Some(VoterWeightAction::CastVote);
    voter_weight_record.weight_action_target = Some(ctx.accounts.proposal.key());

    Ok(())
}
