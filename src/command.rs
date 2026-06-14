use crate::error::PlayerId;

/// Commands issued by the engine to the external dealer process.
///
/// The engine never touches cards directly. Instead it emits these commands
/// which the dealer must execute and respond to with [`GameEvent`](crate::GameEvent).
///
/// # Examples
///
/// ```rust
/// use poker_engine::{Game, GameCommand, GameConfig, GameResponse};
///
/// let mut game = Game::new(GameConfig::default());
/// game.add_player(1, 10000).unwrap();
/// game.add_player(2, 10000).unwrap();
///
/// let resp = game.start_hand().unwrap();
/// match resp {
///     GameResponse::DealerCommand(GameCommand::DealHoleCards { player_ids }) => {
///         assert_eq!(player_ids.len(), 2);
///     }
///     _ => panic!("expected DealerCommand"),
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameCommand {
    /// Deal two hole cards to the specified players.
    DealHoleCards {
        /// IDs of players to receive cards, in dealing order.
        player_ids: Vec<PlayerId>,
    },
    /// Reveal community cards (flop: 3, turn: 1, river: 1).
    RevealCommunityCards {
        /// Number of cards to reveal.
        count: u8,
    },
    /// Reveal hole cards of players at showdown.
    RevealPlayerCards {
        /// IDs of players whose cards must be revealed.
        player_ids: Vec<PlayerId>,
    },
}

/// Available actions for a player.
///
/// Returned as part of [`GameResponse::PlayerTurn`] to inform the frontend
/// or AI what actions are valid and the relevant amounts.
///
/// # Examples
///
/// ```rust
/// use poker_engine::{Game, GameConfig, PlayerAction, GameResponse, GameCommand};
///
/// let mut game = Game::new(GameConfig::default());
/// game.add_player(1, 10000).unwrap();
/// game.add_player(2, 10000).unwrap();
/// game.start_hand().unwrap();
///
/// // Simulate dealer confirming cards dealt
/// game.handle_event(poker_engine::GameEvent::HoleCardsDealt { player_id: 1 }).unwrap();
/// game.handle_event(poker_engine::GameEvent::HoleCardsDealt { player_id: 2 }).unwrap();
///
/// // game_response returns PlayerTurn with available actions
/// let active = game.active_player().unwrap();
/// let resp = game.game_response(active, PlayerAction::Call).unwrap();
/// match resp {
///     GameResponse::PlayerTurn { player_id, actions } => {
///         assert!(actions.contains(&poker_engine::AvailableAction::Fold));
///     }
///     _ => panic!("expected PlayerTurn"),
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvailableAction {
    /// Fold — forfeit the current hand.
    Fold,
    /// Check — pass action without betting (only when no bet to match).
    Check,
    /// Call — match the current bet. The u64 is the amount to call.
    Call(u64),
    /// Open betting (no bet to match). The u64 is the minimum bet.
    Bet(u64),
    /// Raise. The u64 is the minimum raise increment.
    Raise(u64),
    /// All-in. The u64 is the amount (all remaining chips).
    AllIn(u64),
    /// Show cards at showdown.
    ShowCards,
}

/// Response from the engine after processing an action or event.
///
/// Either a command for the dealer, information about whose turn it is,
/// or a signal that the game is over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameResponse {
    /// Command(s) to send to the external dealer process.
    DealerCommand(GameCommand),
    /// A player's turn — includes available actions.
    PlayerTurn {
        /// ID of the player who should act.
        player_id: PlayerId,
        /// Available actions for this player.
        actions: Vec<AvailableAction>,
    },
    /// The hand is over (all folded or showdown complete).
    GameOver,
}

/// Actions a player can take during a betting round.
///
/// # Examples
///
/// ```rust
/// use poker_engine::{Game, GameConfig, PlayerAction, GameEvent};
///
/// let mut game = Game::new(GameConfig::default());
/// game.add_player(1, 10000).unwrap();
/// game.add_player(2, 10000).unwrap();
/// game.start_hand().unwrap();
/// game.handle_event(GameEvent::HoleCardsDealt { player_id: 1 }).unwrap();
/// game.handle_event(GameEvent::HoleCardsDealt { player_id: 2 }).unwrap();
///
/// let active = game.active_player().unwrap();
/// game.game_response(active, PlayerAction::Call).unwrap();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerAction {
    /// Fold — forfeit the current hand.
    Fold,
    /// Check — pass action without betting (only when no bet to match).
    Check,
    /// Call — match the current bet.
    Call,
    /// Raise by the specified amount (must be at least big blind).
    Raise(u64),
    /// All-in — bet all remaining chips.
    AllIn,
}
