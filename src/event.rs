use crate::error::PlayerId;
use crate::hand::HandScore;

/// Events sent by the external dealer back to the engine.
///
/// After receiving a [`GameCommand`](crate::GameCommand), the dealer executes
/// it and responds with one of these events containing the hand evaluation scores.
///
/// # Examples
///
/// ```rust
/// use poker_engine::{Game, GameConfig, GameEvent};
///
/// let mut game = Game::new(GameConfig::default());
/// game.add_player(1).unwrap();
/// game.add_player(2).unwrap();
/// game.start_hand().unwrap();
///
/// // Dealer responds with hand score after dealing hole cards
/// let event = GameEvent::HoleCardsDealt { player_id: 1, score: 8500000 };
/// game.handle_event(event).unwrap();
/// ```
#[derive(Debug, Clone)]
pub enum GameEvent {
    /// Hole cards have been dealt to a player. Contains the hand score evaluated by the dealer.
    HoleCardsDealt {
        /// ID of the player who received cards.
        player_id: PlayerId,
        /// Hand score evaluated by the dealer.
        score: HandScore,
    },
    /// Community cards have been revealed. Contains updated hand scores for all active players.
    CommunityCardsRevealed {
        /// Updated hand scores for each active player.
        scores: Vec<(PlayerId, HandScore)>,
    },
    /// Player cards have been revealed at showdown. Contains the final hand score.
    PlayerCardsRevealed {
        /// ID of the player whose cards were revealed.
        player_id: PlayerId,
        /// Final hand score evaluated by the dealer.
        score: HandScore,
    },
    /// An error occurred during card processing.
    Error(String),
}
