use crate::error::PlayerId;
use crate::hand::HandScore;

#[derive(Debug, Clone)]
pub enum GameEvent {
    HoleCardsDealt {
        player_id: PlayerId,
        score: HandScore,
    },
    CommunityCardsRevealed {
        scores: Vec<(PlayerId, HandScore)>,
    },
    PlayerCardsRevealed {
        player_id: PlayerId,
        score: HandScore,
    },
    Error(String),
}
