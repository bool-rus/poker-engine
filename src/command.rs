use crate::error::PlayerId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameCommand {
    DealHoleCards {
        player_id: PlayerId,
    },
    RevealCommunityCards {
        count: u8,
    },
    RevealPlayerCards {
        player_ids: Vec<PlayerId>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerAction {
    Fold,
    Check,
    Call,
    Raise(u64),
    AllIn,
}
