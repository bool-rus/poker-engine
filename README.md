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
2. Dealer executes commands, evaluates hands, sends back `GameEvent` with `HandScore` (u64).
3. Engine uses scores to determine winners, manage pots, advance phases.

## Usage

```rust
use poker_engine::{Game, GameConfig, PlayerAction, GameCommand, GameEvent};

let config = GameConfig::default();
let mut game = Game::new(config);

// Add players
game.add_player(1).unwrap();
game.add_player(2).unwrap();

// Start a hand — engine returns commands for the dealer
let cmds = game.start_hand().unwrap();
// cmds: [DealHoleCards{player_id:1}, DealHoleCards{player_id:2}]

// Dealer processes cards and returns scores
game.handle_event(GameEvent::HoleCardsDealt { player_id: 1, score: 8500000 }).unwrap();
game.handle_event(GameEvent::HoleCardsDealt { player_id: 2, score: 4200000 }).unwrap();

// Players act
let active = game.active_player().unwrap();
game.player_action(active, PlayerAction::Call).unwrap();
let other = if active == 1 { 2 } else { 1 };
game.player_action(other, PlayerAction::Check).unwrap();

// Now in Flop phase — engine returns RevealCommunityCards{count: 3}
assert_eq!(game.phase(), poker_engine::GamePhase::Flop);
```

## Game Flow

```
GameCommand::DealHoleCards      ──> Dealer deals 2 cards per player
GameEvent::HoleCardsDealt       <── Dealer returns hand scores
Player actions (fold/call/raise) ──> Engine manages betting
GameCommand::RevealCommunityCards ──> Dealer reveals flop/turn/river
GameEvent::CommunityCardsRevealed <── Dealer returns updated scores
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
cargo test          # Run all tests (54 unit + 13 doc tests)
cargo test --doc    # Run doc tests only
```

## License

MIT
