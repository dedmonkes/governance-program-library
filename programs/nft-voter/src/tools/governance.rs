use crate::{error::NftVoterError, state::voter_weight_record::VoterWeightRecord};
use anchor_lang::{prelude::*, Id, Key};
use solana_program::{instruction::Instruction, msg};
use spl_governance::state::{
    proposal::{get_proposal_data, ProposalV2, VoteType},
    proposal_transaction::get_proposal_transaction_data_for_proposal,
    token_owner_record, vote_record,
};

use std::str::FromStr;

use super::phase_protocol::{PhaseProtocolProgram, PHASE_VOTE_DISCRIMATORS};

#[derive(Debug, Clone)]
pub struct DedSplGovernanceProgram;

impl anchor_lang::Id for DedSplGovernanceProgram {
    fn id() -> Pubkey {
        Pubkey::from_str("8CeHquv1qaM4yg1QtXDj8ZmanNxATSEcSi3xWuprecMd").unwrap()
    }
}

pub fn get_vote_record_address(
    program_id: &Pubkey,
    realm: &Pubkey,
    governing_token_mint: &Pubkey,
    governing_token_owner: &Pubkey,
    proposal: &Pubkey,
) -> Pubkey {
    let token_owner_record_key = token_owner_record::get_token_owner_record_address(
        program_id,
        realm,
        governing_token_mint,
        governing_token_owner,
    );

    vote_record::get_vote_record_address(program_id, proposal, &token_owner_record_key)
}

pub fn is_phase_option(proposal: &ProposalV2) -> bool {
    if proposal.options.len() != 1
        || proposal.options[0].transactions_count != 1
        || proposal.options[0].label != "Reject"
        || proposal.vote_type != VoteType::SingleChoice
    {
        return false;
    }

    true
}

pub fn add_voter_weight(
    proposal_info: &AccountInfo,
    governance_program_id: &Pubkey,
    proposal_transaction_info: Option<&AccountInfo>,
    cast_vote_ix: Option<Instruction>,
    voter_weight: u64,
    voter_weight_record: &mut Account<VoterWeightRecord>,
) -> Result<()> {
    let proposal = get_proposal_data(governance_program_id, proposal_info)?;
    if let Some(vote_ix) = cast_vote_ix {
        if is_phase_option(&proposal) {

            if !proposal_transaction_info.is_some() {
                return Err(NftVoterError::MustIncludeProposalTransactionForPhaseVotes.into());
            }

            if let Some(prop_info) = proposal_transaction_info {

                let is_phase =
                    is_phase_transaction(&prop_info, &governance_program_id, &proposal_info.key())?;

                if is_phase {
                    if vote_ix.data.as_slice() != [13, 0, 1, 0, 0, 0, 0, 100] {
                        msg!("Not an approve option for vote setting weight to 0");
                        voter_weight_record.voter_weight = 0;
                        return Ok(())
                    }
                }
            }
        }
    }
    voter_weight_record.voter_weight = voter_weight_record
        .voter_weight
        .checked_add(voter_weight)
        .unwrap();

    Ok(())
}

pub fn is_phase_transaction(
    proposal_transaction_info: &AccountInfo,
    governance_program_id: &Pubkey,
    proposal_key: &Pubkey,
) -> Result<bool> {
    let proposal_transaction = get_proposal_transaction_data_for_proposal(
        &governance_program_id,
        &proposal_transaction_info,
        &proposal_key,
    )?;

    if proposal_transaction.instructions.len() != 1 {
        return Ok(false);
    }

    let instruction = &proposal_transaction.instructions[0];

    let mut discrimanator: [u8; 8] = Default::default();
    discrimanator.copy_from_slice(&instruction.data[0..8]);

    if !PHASE_VOTE_DISCRIMATORS.contains(&discrimanator) {
        return Ok(false);
    }

    if proposal_transaction.instructions[0].program_id != PhaseProtocolProgram::id() {
        return Ok(false);
    }

    Ok(true)
}
