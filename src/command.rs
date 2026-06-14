use crate::error::PlayerId;
use crate::game::GamePhase;

/// Commands issued by the engine to the external dealer process.
///
/// The engine never touches cards directly. Instead it emits these commands
/// which the dealer must execute and respond to with [`GameEvent`](crate::GameEvent).
///
/// # Examples
///
/// ```rust
/// use poker_engine::{Game, GameCommand, GameConfig};
///
/// let mut game = Game::new(GameConfig::default());
/// game.add_player(1, 10000).unwrap();
/// game.add_player(2, 10000).unwrap();
///
/// let cmds = game.start_hand().unwrap();
/// assert!(cmds.contains(&GameCommand::DealHoleCards { player_id: 1 }));
/// assert!(cmds.contains(&GameCommand::DealHoleCards { player_id: 2 }));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameCommand {
    /// Deal two hole cards to the specified player.
    DealHoleCards {
        /// ID of the player receiving cards.
        player_id: PlayerId,
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

/// Available actions for a player during a betting round.
///
/// Returned by [`Game::player_actions`](crate::Game::player_actions) to inform
/// the frontend or AI what actions are valid for the current player.
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
/// let actions = game.player_actions(active).unwrap();
/// assert!(actions.can_fold);
/// assert!(!actions.can_check); // must call or raise pre-flop
/// assert!(actions.can_call);
/// assert!(actions.can_raise);
/// assert!(actions.can_all_in);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerActions {
    /// The player these actions apply to.
    pub player_id: PlayerId,
    /// Current game phase.
    pub phase: GamePhase,
    /// Player can always fold.
    pub can_fold: bool,
    /// Player can check (no bet to match).
    pub can_check: bool,
    /// Player can call the current bet.
    pub can_call: bool,
    /// Amount needed to call (may exceed player's chips for all-in call).
    pub call_amount: u64,
    /// Player can raise.
    pub can_raise: bool,
    /// Minimum raise increment.
    pub min_raise: u64,
    /// Maximum raise amount (player's remaining chips after calling).
    pub max_raise: u64,
    /// Player can go all-in.
    pub can_all_in: bool,
    /// All-in amount (all remaining chips).
    pub all_in_amount: u64,
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
/// game.player_action(active, PlayerAction::Call).unwrap();
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
