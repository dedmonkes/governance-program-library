use std::str::FromStr;

use anchor_lang::prelude::Pubkey;

pub const REVERT_STAGED_APPROVE_PHASE_DATA: [u8; 8] = [138, 129, 29, 173, 82, 210, 89, 168];
pub const REVERT_STAGED_COMPLETE_PHASE_DATA: [u8; 8] = [180, 24, 255, 165, 254, 132, 50, 222];
pub const REVERT_STAGED_RESOLUTION_ROADMAP: [u8; 8] = [233, 116, 43, 231, 46, 145, 37, 131];

pub const PHASE_VOTE_DISCRIMATORS: [[u8; 8]; 3] = [
    REVERT_STAGED_APPROVE_PHASE_DATA,
    REVERT_STAGED_COMPLETE_PHASE_DATA,
    REVERT_STAGED_RESOLUTION_ROADMAP,
];
#[derive(Debug, Clone)]
pub struct PhaseProtocolProgram;

impl anchor_lang::Id for PhaseProtocolProgram {
    fn id() -> Pubkey {
        Pubkey::from_str("Di92bTGdAUgdfKJYAxC5dX5PJUqmqz3hP84LrHeHXz6M").unwrap()
    }
}
