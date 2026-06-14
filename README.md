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
use poker_engine::{Game, GameConfig, PlayerAction, GameEvent};

let mut game = Game::new(GameConfig::default());
game.add_player(1).unwrap();
game.add_player(2).unwrap();

// --- Pre-flop ---
let cmds = game.start_hand().unwrap();
// cmds: [DealHoleCards{1}, DealHoleCards{2}]

// Dealer confirms cards dealt (no scores — engine doesn't know card values)
game.handle_event(GameEvent::HoleCardsDealt { player_id: 1 }).unwrap();
game.handle_event(GameEvent::HoleCardsDealt { player_id: 2 }).unwrap();

let active = game.active_player().unwrap();
let other = if active == 1 { 2 } else { 1 };
game.player_action(active, PlayerAction::Call).unwrap();
game.player_action(other, PlayerAction::Check).unwrap();
assert_eq!(game.phase(), poker_engine::GamePhase::Flop);

// --- Flop ---
// Engine returns RevealCommunityCards{count: 3}
game.handle_event(GameEvent::CommunityCardsRevealed).unwrap();
let a = game.active_player().unwrap();
let o = if a == 1 { 2 } else { 1 };
game.player_action(a, PlayerAction::Check).unwrap();
game.player_action(o, PlayerAction::Check).unwrap();
assert_eq!(game.phase(), poker_engine::GamePhase::Turn);

// --- Turn ---
game.handle_event(GameEvent::CommunityCardsRevealed).unwrap();
let a = game.active_player().unwrap();
let o = if a == 1 { 2 } else { 1 };
game.player_action(a, PlayerAction::Check).unwrap();
game.player_action(o, PlayerAction::Check).unwrap();
assert_eq!(game.phase(), poker_engine::GamePhase::River);

// --- River ---
game.handle_event(GameEvent::CommunityCardsRevealed).unwrap();
let a = game.active_player().unwrap();
let o = if a == 1 { 2 } else { 1 };
game.player_action(a, PlayerAction::Check).unwrap();
game.player_action(o, PlayerAction::Check).unwrap();
assert_eq!(game.phase(), poker_engine::GamePhase::Showdown);

// --- Showdown ---
// Dealer evaluates hands and sends scores
game.handle_event(GameEvent::PlayerCardsRevealed { player_id: 1, score: 500 }).unwrap();
game.handle_event(GameEvent::PlayerCardsRevealed { player_id: 2, score: 800 }).unwrap();
assert_eq!(game.phase(), poker_engine::GamePhase::GameOver);
```

## All-in Showdown

When all players go all-in, the engine auto-advances through community card phases to showdown:

```rust
use poker_engine::{Game, GameConfig, PlayerAction, GameEvent};

let config = GameConfig {
    small_blind: 50,
    big_blind: 100,
    starting_chips: 1000,
    max_players: 9,
    min_players: 2,
    allow_rebuy: false,
    rebuy_amount: None,
};

let mut game = Game::new(config);
game.add_player(1).unwrap();
game.add_player(2).unwrap();
game.add_player(3).unwrap();

// --- Pre-flop: all players go all-in ---
game.start_hand().unwrap();
game.handle_event(GameEvent::HoleCardsDealt { player_id: 1 }).unwrap();
game.handle_event(GameEvent::HoleCardsDealt { player_id: 2 }).unwrap();
game.handle_event(GameEvent::HoleCardsDealt { player_id: 3 }).unwrap();

let a = game.active_player().unwrap();
game.player_action(a, PlayerAction::AllIn).unwrap();
let a = game.active_player().unwrap();
game.player_action(a, PlayerAction::AllIn).unwrap();
let a = game.active_player().unwrap();
game.player_action(a, PlayerAction::AllIn).unwrap();

// --- Auto-advance: engine skips Flop/Turn/River, goes straight to Showdown ---
while game.phase() != poker_engine::GamePhase::Showdown
    && game.phase() != poker_engine::GamePhase::GameOver
{
    let _ = game.handle_event(GameEvent::CommunityCardsRevealed);
}
assert_eq!(game.phase(), poker_engine::GamePhase::Showdown);
assert_eq!(game.pot_total(), 3000); // 3 × 1000 chips

// --- Showdown: dealer sends scores, winner takes the pot ---
game.handle_event(GameEvent::PlayerCardsRevealed { player_id: 1, score: 100 }).unwrap();
game.handle_event(GameEvent::PlayerCardsRevealed { player_id: 2, score: 200 }).unwrap();
game.handle_event(GameEvent::PlayerCardsRevealed { player_id: 3, score: 300 }).unwrap();
assert_eq!(game.phase(), poker_engine::GamePhase::GameOver);

let winner = game.all_players().iter().find(|p| p.id == 3).unwrap();
assert_eq!(winner.chips, 3000); // player 3 wins the entire pot
```

## Game Flow

```
GameCommand::DealHoleCards      ──> Dealer deals 2 cards per player
GameEvent::HoleCardsDealt       <── Dealer confirms cards dealt
Player actions (fold/call/raise) ──> Engine manages betting
GameCommand::RevealCommunityCards ──> Dealer reveals flop/turn/river
GameEvent::CommunityCardsRevealed <── Dealer confirms cards revealed
GameCommand::RevealPlayerCards  ──> Dealer shows cards at showdown
GameEvent::PlayerCardsRevealed   <── Dealer returns final scores
```

## Features

- **Command/Event architecture** — clean separation between game logic and card handling
- **Full betting logic** — fold, check, call, raise, all-in with proper minimum raise enforcement
- **Pot management** — main pot and side pots for multi-way all-in
- **Dynamic players** — add/remove players between hands, sit out/in, rebuy
- **Dealer rotation** — automatic dealer button movement between hands
- **Configurable** — blinds, starting chips, min/max players, rebuy settings

## Configuration

```rust
use poker_engine::GameConfig;

let config = GameConfig {
    small_blind: 25,
    big_blind: 50,
    starting_chips: 5000,
    max_players: 6,
    min_players: 2,
    allow_rebuy: true,
    rebuy_amount: Some(5000),
};
```

## Player Management

```rust
game.add_player(1)?;           // Add player with starting chips
game.remove_player(1)?;        // Remove player between hands
game.sit_out(1)?;              // Auto-fold until sit_in
game.sit_in(1)?;               // Rejoin next hand
game.rebuy(1, 5000)?;          // Add chips (up to rebuy_amount)
```

## API Reference

| Method | Description |
|--------|-------------|
| `Game::new(config)` | Create a new game |
| `Game::add_player(id)` | Add player to table |
| `Game::remove_player(id)` | Remove player from table |
| `Game::start_hand()` | Start a new hand, returns `Vec<GameCommand>` |
| `Game::handle_event(event)` | Process dealer response |
| `Game::player_action(id, action)` | Execute a player action |
| `Game::active_player()` | Get ID of player who should act |
| `Game::phase()` | Current game phase |
| `Game::pot_total()` | Total chips in the pot |

## Testing

```bash
cargo test          # Run all tests (63 unit + 13 doc tests)
cargo test --doc    # Run doc tests only
```

## License

MIT
