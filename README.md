# poker-engine

Texas Hold'em poker engine for Rust with Command/Event architecture.

The engine manages game state, betting rounds, pots, and player actions. Card dealing and hand evaluation are delegated to an external dealer process.

## Architecture

```
┌─────────────┐   GameCommand    ┌──────────────┐
│   Engine    │ ──────────────>  │    Dealer    │
│  (poker)    │ <──────────────  │  (external)  │
└─────────────┘    GameEvent     └──────────────┘
```

1. Engine issues `GameCommand` — deal cards, reveal community cards, reveal player cards.
2. Dealer executes commands, evaluates hands, sends back `GameEvent`. Only `PlayerCardsRevealed` carries a `HandScore` (u64) — the engine only needs scores at showdown.
3. Engine uses scores to determine winners, manage pots, advance phases.

## Usage

```rust
use poker_engine::{Game, GameConfig, PlayerAction, GameCommand, GameEvent, GameResponse};

let mut game = Game::new(GameConfig::default());
game.add_player(1, 10000).unwrap();
game.add_player(2, 10000).unwrap();

// --- Pre-flop ---
let resp = game.start_hand().unwrap();
// resp: DealerCommand(DealHoleCards { player_ids: [1, 2] })
if let GameResponse::DealerCommand(GameCommand::DealHoleCards { player_ids }) = resp {
    for id in player_ids {
        game.handle_event(GameEvent::HoleCardsDealt { player_id: id }).unwrap();
    }
}

// Execute actions — engine returns GameResponse with available actions
let active = game.active_player().unwrap();
let other = if active == 1 { 2 } else { 1 };
game.game_response(active, PlayerAction::Call).unwrap();
let resp = game.game_response(other, PlayerAction::Check).unwrap();
// resp: DealerCommand(RevealCommunityCards { count: 3 }) — flop
assert_eq!(game.phase(), poker_engine::GamePhase::Flop);

// --- Flop ---
game.handle_event(GameEvent::CommunityCardsRevealed).unwrap();
let a = game.active_player().unwrap();
let o = if a == 1 { 2 } else { 1 };
game.game_response(a, PlayerAction::Check).unwrap();
game.game_response(o, PlayerAction::Check).unwrap();
assert_eq!(game.phase(), poker_engine::GamePhase::Turn);

// --- Turn ---
game.handle_event(GameEvent::CommunityCardsRevealed).unwrap();
let a = game.active_player().unwrap();
let o = if a == 1 { 2 } else { 1 };
game.game_response(a, PlayerAction::Check).unwrap();
game.game_response(o, PlayerAction::Check).unwrap();
assert_eq!(game.phase(), poker_engine::GamePhase::River);

// --- River ---
game.handle_event(GameEvent::CommunityCardsRevealed).unwrap();
let a = game.active_player().unwrap();
let o = if a == 1 { 2 } else { 1 };
game.game_response(a, PlayerAction::Check).unwrap();
game.game_response(o, PlayerAction::Check).unwrap();
assert_eq!(game.phase(), poker_engine::GamePhase::Showdown);

// --- Showdown ---
game.handle_event(GameEvent::PlayerCardsRevealed { player_id: 1, score: 500 }).unwrap();
game.handle_event(GameEvent::PlayerCardsRevealed { player_id: 2, score: 800 }).unwrap();
assert_eq!(game.phase(), poker_engine::GamePhase::GameOver);
```

## Available Actions

Every call to `start_hand()`, `handle_event()`, or `game_response()` returns a `GameResponse`:

```rust
pub enum GameResponse {
    DealerCommand(GameCommand),                    // send to dealer
    PlayerTurn { player_id, actions: Vec<AvailableAction> },  // whose turn
    GameOver,                                      // hand complete
}

pub enum AvailableAction {
    Fold,
    Check,
    Call(u64),       // amount to call
    Bet(u64),        // minimum open bet
    Raise(u64),      // minimum raise increment
    AllIn(u64),      // all-in amount
    ShowCards,
}
```

```rust
use poker_engine::{Game, GameConfig, PlayerAction, GameCommand, GameEvent, GameResponse};

let mut game = Game::new(GameConfig::default());
game.add_player(1, 10000).unwrap();
game.add_player(2, 10000).unwrap();
let resp = game.start_hand().unwrap();
if let GameResponse::DealerCommand(GameCommand::DealHoleCards { player_ids }) = resp {
    for id in player_ids {
        let resp = game.handle_event(GameEvent::HoleCardsDealt { player_id: id }).unwrap();
        if let GameResponse::PlayerTurn { player_id, actions } = resp {
            // First player to act — check available actions
            assert!(actions.contains(&poker_engine::AvailableAction::Fold));
            assert!(actions.contains(&poker_engine::AvailableAction::Call(50)));
            assert!(actions.contains(&poker_engine::AvailableAction::Raise(100)));
            assert!(actions.contains(&poker_engine::AvailableAction::AllIn(10000)));
        }
    }
}
```

## All-in Showdown

When all players go all-in, the engine auto-advances through community card phases to showdown:

```rust
use poker_engine::{Game, GameConfig, PlayerAction, GameCommand, GameEvent, GameResponse};

let mut game = Game::new(GameConfig {
    small_blind: 50,
    big_blind: 100,
    max_players: 9,
    min_players: 2,
    allow_rebuy: false,
});

game.add_player(1, 1000).unwrap();
game.add_player(2, 1000).unwrap();
game.add_player(3, 1000).unwrap();

// --- Pre-flop: all players go all-in ---
let resp = game.start_hand().unwrap();
if let GameResponse::DealerCommand(GameCommand::DealHoleCards { player_ids }) = resp {
    for id in player_ids {
        game.handle_event(GameEvent::HoleCardsDealt { player_id: id }).unwrap();
    }
}

let a = game.active_player().unwrap();
game.game_response(a, PlayerAction::AllIn).unwrap();
let a = game.active_player().unwrap();
game.game_response(a, PlayerAction::AllIn).unwrap();
let a = game.active_player().unwrap();
game.game_response(a, PlayerAction::AllIn).unwrap();

// --- Auto-advance: engine skips Flop/Turn/River, goes straight to Showdown ---
while game.phase() != poker_engine::GamePhase::Showdown
    && game.phase() != poker_engine::GamePhase::GameOver
{
    let _ = game.handle_event(GameEvent::CommunityCardsRevealed);
}
assert_eq!(game.phase(), poker_engine::GamePhase::Showdown);
assert_eq!(game.pot_total(), 3000);

// --- Showdown: dealer sends scores, winner takes the pot ---
game.handle_event(GameEvent::PlayerCardsRevealed { player_id: 1, score: 100 }).unwrap();
game.handle_event(GameEvent::PlayerCardsRevealed { player_id: 2, score: 200 }).unwrap();
game.handle_event(GameEvent::PlayerCardsRevealed { player_id: 3, score: 300 }).unwrap();
assert_eq!(game.phase(), poker_engine::GamePhase::GameOver);

let winner = game.all_players().iter().find(|p| p.id == 3).unwrap();
assert_eq!(winner.chips, 3000);
```

## Game Flow

```
start_hand()           ──> GameResponse::DealerCommand(DealHoleCards)
handle_event(HoleCardsDealt) <── GameResponse::PlayerTurn (whose turn)
game_response(id, action)  ──> GameResponse::PlayerTurn or DealerCommand
handle_event(CommunityCardsRevealed) <── GameResponse::PlayerTurn or DealerCommand
handle_event(PlayerCardsRevealed) <── GameResponse::GameOver
```

## Features

- **Command/Event architecture** — clean separation between game logic and card handling
- **Full betting logic** — fold, check, call, raise, all-in with proper minimum raise enforcement
- **Pot management** — main pot and side pots for multi-way all-in
- **Dynamic players** — add/remove players between hands, sit out/in, rebuy
- **Dealer rotation** — automatic dealer button movement between hands
- **Available actions** — every response includes valid actions with amounts
- **Configurable** — blinds, min/max players, rebuy settings

## Configuration

```rust
use poker_engine::GameConfig;

let config = GameConfig {
    small_blind: 25,
    big_blind: 50,
    max_players: 6,
    min_players: 2,
    allow_rebuy: true,
};
```

## Player Management

```rust
game.add_player(1, 10000)?;     // Add player with chip stack
game.remove_player(1)?;         // Remove player between hands
game.sit_out(1)?;               // Auto-fold until sit_in
game.sit_in(1)?;                // Rejoin next hand
game.rebuy(1, 5000)?;           // Add chips
```

## API Reference

| Method | Returns | Description |
|--------|---------|-------------|
| `Game::new(config)` | `Game` | Create a new game |
| `Game::add_player(id, chips)` | `Result<(), PokerError>` | Add player with chip stack |
| `Game::remove_player(id)` | `Result<(), PokerError>` | Remove player from table |
| `Game::start_hand()` | `Result<GameResponse, PokerError>` | Start a new hand |
| `Game::handle_event(event)` | `Result<GameResponse, PokerError>` | Process dealer response |
| `Game::game_response(id, action)` | `Result<GameResponse, PokerError>` | Execute a player action |
| `Game::active_player()` | `Option<PlayerId>` | Get ID of player who should act |
| `Game::phase()` | `GamePhase` | Current game phase |
| `Game::pot_total()` | `u64` | Total chips in the pot |

## Types

| Type | Description |
|------|-------------|
| `GameResponse` | Response enum: `DealerCommand`, `PlayerTurn`, `GameOver` |
| `GameCommand` | Commands: `DealHoleCards`, `RevealCommunityCards`, `RevealPlayerCards` |
| `AvailableAction` | Available actions: `Fold`, `Check`, `Call`, `Bet`, `Raise`, `AllIn`, `ShowCards` |
| `PlayerAction` | Player input: `Fold`, `Check`, `Call`, `Raise(u64)`, `AllIn` |
| `GameEvent` | Dealer events: `HoleCardsDealt`, `CommunityCardsRevealed`, `PlayerCardsRevealed` |

## Testing

```bash
cargo test          # Run all tests (74 unit + 14 doc tests)
cargo test --doc    # Run doc tests only
```

## License

MIT
