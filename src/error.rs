use std::fmt;

pub type PlayerId = u64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PokerError {
    InvalidAction(String),
    PlayerNotFound(PlayerId),
    PlayerAlreadyAtTable(PlayerId),
    PlayerNotInHand(PlayerId),
    NotYourTurn { expected: PlayerId, got: PlayerId },
    GameInProgress,
    NotEnoughPlayers { required: usize, current: usize },
    TableFull { max: usize },
    NotEnoughChips { player: PlayerId, available: u64, required: u64 },
    RaiseBelowMinimum { minimum: u64, attempted: u64 },
    GameNotInProgress,
    CannotStartHand,
    PlayerIsAllIn(PlayerId),
    PlayerSittingOut(PlayerId),
    InvalidRebuy { player: PlayerId, amount: u64 },
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
