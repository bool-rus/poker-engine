use std::fmt;

/// Unique identifier for a player at the table.
pub type PlayerId = u64;

/// Errors that can occur during game operations.
///
/// # Examples
///
/// ```rust
/// use poker_engine::{Game, GameConfig, PokerError};
///
/// let config = GameConfig { max_players: 2, ..GameConfig::default() };
/// let mut game = Game::new(config);
/// game.add_player(1, 10000).unwrap();
/// game.add_player(2, 10000).unwrap();
/// assert_eq!(game.add_player(3, 10000), Err(PokerError::TableFull { max: 2 }));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PokerError {
    /// The requested action is not valid in the current game state.
    InvalidAction(String),
    /// The specified player was not found at the table.
    PlayerNotFound(PlayerId),
    /// A player with this ID is already at the table.
    PlayerAlreadyAtTable(PlayerId),
    /// The player is not in the current hand.
    PlayerNotInHand(PlayerId),
    /// It is not this player's turn to act.
    NotYourTurn {
        /// The player whose turn it is.
        expected: PlayerId,
        /// The player who attempted to act.
        got: PlayerId,
    },
    /// Cannot modify the table while a hand is in progress.
    GameInProgress,
    /// Not enough players to start a hand.
    NotEnoughPlayers {
        /// Minimum required players.
        required: usize,
        /// Current number of eligible players.
        current: usize,
    },
    /// The table has reached its maximum capacity.
    TableFull {
        /// Maximum number of players.
        max: usize,
    },
    /// The player does not have enough chips for the requested action.
    NotEnoughChips {
        /// The player's ID.
        player: PlayerId,
        /// Available chips.
        available: u64,
        /// Required chips.
        required: u64,
    },
    /// A raise is below the minimum allowed amount.
    RaiseBelowMinimum {
        /// Minimum raise amount.
        minimum: u64,
        /// Attempted raise amount.
        attempted: u64,
    },
    /// No hand is currently in progress.
    GameNotInProgress,
    /// Cannot start a new hand (not enough eligible players).
    CannotStartHand,
    /// The player is all-in and cannot act further.
    PlayerIsAllIn(PlayerId),
    /// The player is sitting out and cannot participate.
    PlayerSittingOut(PlayerId),
    /// The rebuy amount is invalid.
    InvalidRebuy {
        /// The player's ID.
        player: PlayerId,
        /// Attempted rebuy amount.
        amount: u64,
    },
}

impl fmt::Display for PokerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PokerError::InvalidAction(msg) => write!(f, "Invalid action: {}", msg),
            PokerError::PlayerNotFound(id) => write!(f, "Player {} not found", id),
            PokerError::PlayerAlreadyAtTable(id) => write!(f, "Player {} already at table", id),
            PokerError::PlayerNotInHand(id) => write!(f, "Player {} not in current hand", id),
            PokerError::NotYourTurn { expected, got } => {
                write!(f, "Not your turn: expected player {}, got {}", expected, got)
            }
            PokerError::GameInProgress => write!(f, "Cannot modify table while game is in progress"),
            PokerError::NotEnoughPlayers { required, current } => {
                write!(f, "Not enough players: need {}, have {}", required, current)
            }
            PokerError::TableFull { max } => write!(f, "Table is full (max {})", max),
            PokerError::NotEnoughChips { player, available, required } => {
                write!(f, "Player {} has {} chips, needs {}", player, available, required)
            }
            PokerError::RaiseBelowMinimum { minimum, attempted } => {
                write!(f, "Raise {} below minimum {}", attempted, minimum)
            }
            PokerError::GameNotInProgress => write!(f, "No game in progress"),
            PokerError::CannotStartHand => write!(f, "Cannot start hand"),
            PokerError::PlayerIsAllIn(id) => write!(f, "Player {} is all-in", id),
            PokerError::PlayerSittingOut(id) => write!(f, "Player {} is sitting out", id),
            PokerError::InvalidRebuy { player, amount } => {
                write!(f, "Invalid rebuy for player {}: {}", player, amount)
            }
        }
    }
}

impl std::error::Error for PokerError {}
