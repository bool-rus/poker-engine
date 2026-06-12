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
//! let mut game = Game::new(GameConfig::default());
//! game.add_player(1).unwrap();
//! game.add_player(2).unwrap();
//!
//! // --- Pre-flop ---
//! let cmds = game.start_hand().unwrap();
//! // cmds: [DealHoleCards{1}, DealHoleCards{2}]
//!
//! game.handle_event(GameEvent::HoleCardsDealt { player_id: 1 }).unwrap();
//! game.handle_event(GameEvent::HoleCardsDealt { player_id: 2 }).unwrap();
//!
//! let active = game.active_player().unwrap();
//! let other = if active == 1 { 2 } else { 1 };
//! game.player_action(active, PlayerAction::Call).unwrap();
//! game.player_action(other, PlayerAction::Check).unwrap();
//! assert_eq!(game.phase(), poker_engine::GamePhase::Flop);
//!
//! // --- Flop ---
//! // Engine requests RevealCommunityCards{count: 3}
//! game.handle_event(GameEvent::CommunityCardsRevealed).unwrap();
//!
//! let a = game.active_player().unwrap();
//! let o = if a == 1 { 2 } else { 1 };
//! game.player_action(a, PlayerAction::Check).unwrap();
//! game.player_action(o, PlayerAction::Check).unwrap();
//! assert_eq!(game.phase(), poker_engine::GamePhase::Turn);
//!
//! // --- Turn ---
//! game.handle_event(GameEvent::CommunityCardsRevealed).unwrap();
//! let a = game.active_player().unwrap();
//! let o = if a == 1 { 2 } else { 1 };
//! game.player_action(a, PlayerAction::Check).unwrap();
//! game.player_action(o, PlayerAction::Check).unwrap();
//! assert_eq!(game.phase(), poker_engine::GamePhase::River);
//!
//! // --- River ---
//! game.handle_event(GameEvent::CommunityCardsRevealed).unwrap();
//! let a = game.active_player().unwrap();
//! let o = if a == 1 { 2 } else { 1 };
//! game.player_action(a, PlayerAction::Check).unwrap();
//! game.player_action(o, PlayerAction::Check).unwrap();
//! assert_eq!(game.phase(), poker_engine::GamePhase::Showdown);
//!
//! // --- Showdown ---
//! // Dealer evaluates hands and sends scores
//! game.handle_event(GameEvent::PlayerCardsRevealed { player_id: 1, score: 500 }).unwrap();
//! game.handle_event(GameEvent::PlayerCardsRevealed { player_id: 2, score: 800 }).unwrap();
//! // Hand complete — player 2 wins
//! assert_eq!(game.phase(), poker_engine::GamePhase::GameOver);
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
