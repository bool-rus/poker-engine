use crate::error::PlayerId;
use crate::hand::HandScore;

/// Events sent by the external dealer back to the engine.
///
/// After receiving a [`GameCommand`](crate::GameCommand), the dealer executes
/// it and responds with an event. Only [`PlayerCardsRevealed`] carries a score —
/// the engine needs hand scores only at showdown to determine the winner.
///
/// # Examples
///
/// ```rust
/// use poker_engine::{Game, GameConfig, GameEvent, PlayerAction};
///
/// let mut game = Game::new(GameConfig::default());
/// game.add_player(1).unwrap();
/// game.add_player(2).unwrap();
/// game.start_hand().unwrap();
///
/// // Dealer confirms cards were dealt
/// game.handle_event(GameEvent::HoleCardsDealt { player_id: 1 }).unwrap();
/// game.handle_event(GameEvent::HoleCardsDealt { player_id: 2 }).unwrap();
///
/// // Preflop betting
/// let active = game.active_player().unwrap();
/// game.player_action(active, PlayerAction::Call).unwrap();
/// let other = if active == 1 { 2 } else { 1 };
/// game.player_action(other, PlayerAction::Check).unwrap();
/// ```
#[derive(Debug, Clone)]
pub enum GameEvent {
    /// Hole cards have been dealt to a player.
    ///
    /// Valid only during [`GamePhase::PreFlop`](crate::GamePhase::PreFlop).
    HoleCardsDealt {
        /// ID of the player who received cards.
        player_id: PlayerId,
    },
    /// Community cards have been revealed (flop, turn, or river).
    ///
    /// Valid during [`GamePhase::Flop`](crate::GamePhase::Flop),
    /// [`GamePhase::Turn`](crate::GamePhase::Turn), or
    /// [`GamePhase::River`](crate::GamePhase::River).
    CommunityCardsRevealed,
    /// Player cards have been revealed at showdown with their final hand score.
    ///
    /// Valid only during [`GamePhase::Showdown`](crate::GamePhase::Showdown).
    /// The engine compares scores to determine the winner.
    PlayerCardsRevealed {
        /// ID of the player whose cards were revealed.
        player_id: PlayerId,
        /// Final hand score evaluated by the dealer.
        score: HandScore,
    },
    /// An error occurred during card processing.
    Error(String),
}
