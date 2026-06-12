use crate::error::PlayerId;

/// Player status at the table.
///
/// # Examples
///
/// ```rust
/// use poker_engine::PlayerStatus;
///
/// assert!(PlayerStatus::Active.is_in_game());
/// assert!(PlayerStatus::SittingOut.is_in_game());
/// assert!(!PlayerStatus::Out.is_in_game());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerStatus {
    /// Player is actively participating in hands.
    Active,
    /// Player is temporarily sitting out (auto-folds each hand).
    SittingOut,
    /// Player has left the table.
    Out,
}

impl PlayerStatus {
    /// Returns `true` if the player is still at the table (Active or SittingOut).
    pub fn is_in_game(&self) -> bool {
        matches!(self, PlayerStatus::Active | PlayerStatus::SittingOut)
    }
}

/// State of a single player at the table.
///
/// # Examples
///
/// ```rust
/// use poker_engine::PlayerState;
///
/// let mut p = PlayerState::new(1, 1000);
/// assert_eq!(p.chips, 1000);
/// assert!(!p.all_in);
///
/// p.place_bet(500);
/// assert_eq!(p.chips, 500);
/// assert_eq!(p.bet, 500);
///
/// p.collect_winnings(300);
/// assert_eq!(p.chips, 800);
/// ```
#[derive(Debug, Clone)]
pub struct PlayerState {
    /// Unique player identifier.
    pub id: PlayerId,
    /// Current chip count.
    pub chips: u64,
    /// Bet placed in the current hand.
    pub bet: u64,
    /// Player status (Active, SittingOut, Out).
    pub status: PlayerStatus,
    /// Whether this player is the dealer for the current hand.
    pub is_dealer: bool,
    /// Whether the player wants to rejoin from SittingOut.
    pub wants_in: bool,
    /// Whether the player is all-in.
    pub all_in: bool,
}

impl PlayerState {
    /// Create a new player with the given ID and starting chips.
    pub fn new(id: PlayerId, chips: u64) -> Self {
        Self {
            id,
            chips,
            bet: 0,
            status: PlayerStatus::Active,
            is_dealer: false,
            wants_in: true,
            all_in: false,
        }
    }

    /// Returns `true` if the player is active in the current hand.
    pub fn is_active_in_hand(&self) -> bool {
        self.status == PlayerStatus::Active && (self.chips > 0 || self.bet > 0)
    }

    /// Returns `true` if the player can still act (not all-in, has chips).
    pub fn can_act(&self) -> bool {
        self.status == PlayerStatus::Active && !self.all_in && self.chips > 0
    }

    /// Place a bet. Amount is clamped to available chips. Sets all_in if chips reach 0.
    pub fn place_bet(&mut self, amount: u64) {
        let actual = amount.min(self.chips);
        self.chips -= actual;
        self.bet += actual;
        if self.chips == 0 {
            self.all_in = true;
        }
    }

    /// Add winnings to the player's chip stack.
    pub fn collect_winnings(&mut self, amount: u64) {
        self.chips += amount;
    }

    /// Reset player state for a new hand (clears bet and all_in).
    pub fn reset_for_new_hand(&mut self) {
        self.bet = 0;
        self.all_in = false;
    }
}

#[cfg(test)]
mod tests;
