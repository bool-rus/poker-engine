//! # poker-engine
//!
//! Texas Hold'em poker engine with Command/Event architecture for two-process model.
//!
//! The engine does not know about card values — it manages game state, betting rounds,
//! pots, and player actions. Card dealing and evaluation is delegated to an external
//! dealer process via [`GameCommand`] / [`GameEvent`] messages.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────┐   GameCommand    ┌──────────────┐
//! │   Engine    │ ──────────────>  │    Dealer    │
//! │  (poker)    │ <──────────────  │  (external)  │
//! └─────────────┘    GameEvent     └──────────────┘
//! ```
//!
//! 1. Engine issues [`GameCommand`] — deal cards, reveal community cards, reveal player cards.
//! 2. Dealer executes commands, evaluates hands, sends back [`GameEvent`] with [`HandScore`] (u64).
//! 3. Engine uses scores to determine winners, manage pots, advance phases.
//!
//! ## Quick start
//!
//! ```rust
//! use poker_engine::{Game, GameConfig, PlayerAction, GameCommand, GameEvent};
//!
//! let config = GameConfig::default();
//! let mut game = Game::new(config);
//!
//! // Add players
//! game.add_player(1).unwrap();
//! game.add_player(2).unwrap();
//!
//! // Start a hand — engine returns commands for the dealer
//! let cmds = game.start_hand().unwrap();
//! // cmds contains [DealHoleCards{player_id:1}, DealHoleCards{player_id:2}]
//!
//! // Dealer processes cards and returns scores
//! game.handle_event(GameEvent::HoleCardsDealt { player_id: 1, score: 8500000 }).unwrap();
//! game.handle_event(GameEvent::HoleCardsDealt { player_id: 2, score: 4200000 }).unwrap();
//!
//! // Players act
//! let active = game.active_player().unwrap();
//! game.player_action(active, PlayerAction::Call).unwrap();
//! let other = if active == 1 { 2 } else { 1 };
//! game.player_action(other, PlayerAction::Check).unwrap();
//!
//! // Now in Flop phase — engine returns RevealCommunityCards{count: 3}
//! assert_eq!(game.phase(), poker_engine::GamePhase::Flop);
//! ```

pub mod error;
pub mod hand;
pub mod player;
pub mod command;
pub mod event;
pub mod pot;
pub mod game;

pub use error::{PokerError, PlayerId};
pub use hand::HandScore;
pub use player::{PlayerState, PlayerStatus};
pub use command::{GameCommand, PlayerAction};
pub use event::GameEvent;
pub use pot::Pot;
pub use game::{Game, GameConfig, GamePhase};
