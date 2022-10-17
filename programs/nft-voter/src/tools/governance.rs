use anchor_lang::{prelude::Pubkey, Id};
use solana_program::msg;
use spl_governance::state::{
    proposal::ProposalV2, proposal_transaction::ProposalTransactionV2, token_owner_record,
    vote_record,
};

use std::str::FromStr;

use super::phase_protocol::{PhaseProtocolProgram, PHASE_VOTE_DISCRIMATORS};

#[derive(Debug, Clone)]
pub struct DedSplGovernanceProgram;

impl anchor_lang::Id for DedSplGovernanceProgram {
    fn id() -> Pubkey {
        Pubkey::from_str("8rCUEcjLgKw8YwtgoFjdD9pjdCNxKD9Xc2DPHVw7qZKx").unwrap()
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

pub fn is_phase_vote(proposal_transaction: &ProposalTransactionV2) -> bool {

    if proposal_transaction.instructions.len() != 1 {
        return false;
    }

    let instruction = &proposal_transaction.instructions[0];

    let mut discrimanator: [u8; 8] = Default::default();
    discrimanator.copy_from_slice(&instruction.data[0..8]);

    if !PHASE_VOTE_DISCRIMATORS.contains(&discrimanator) {

        return false;
    }

    if proposal_transaction.instructions[0].program_id != PhaseProtocolProgram::id() {
        return false;
    }


    true
}
