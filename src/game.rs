use crate::error::{PokerError, PlayerId};
use crate::hand::HandScore;
use crate::player::{PlayerState, PlayerStatus};
use crate::command::{GameCommand, PlayerAction};
use crate::event::GameEvent;
use crate::pot::Pot;

/// Phases of a single poker hand.
///
/// # Examples
///
/// ```rust
/// use poker_engine::{Game, GameConfig, GamePhase};
///
/// let mut game = Game::new(GameConfig::default());
/// assert_eq!(game.phase(), GamePhase::WaitingToStart);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GamePhase {
    /// Waiting for the hand to start.
    WaitingToStart,
    /// Pre-flop betting round.
    PreFlop,
    /// Flop betting round (3 community cards revealed).
    Flop,
    /// Turn betting round (4th community card revealed).
    Turn,
    /// River betting round (5th community card revealed).
    River,
    /// Showdown — players reveal cards.
    Showdown,
    /// Hand is over, waiting for the next one.
    GameOver,
}

/// Configuration for a poker game.
///
/// # Examples
///
/// ```rust
/// use poker_engine::GameConfig;
///
/// let config = GameConfig {
///     small_blind: 25,
///     big_blind: 50,
///     starting_chips: 5000,
///     max_players: 6,
///     min_players: 2,
///     allow_rebuy: true,
///     rebuy_amount: Some(5000),
/// };
/// assert_eq!(config.big_blind, 50);
/// ```
#[derive(Debug, Clone)]
pub struct GameConfig {
    /// Small blind amount.
    pub small_blind: u64,
    /// Big blind amount.
    pub big_blind: u64,
    /// Starting chips for each player.
    pub starting_chips: u64,
    /// Maximum number of players at the table.
    pub max_players: usize,
    /// Minimum number of players to start a hand.
    pub min_players: usize,
    /// Whether rebuys are allowed.
    pub allow_rebuy: bool,
    /// Maximum rebuy amount (None = starting_chips).
    pub rebuy_amount: Option<u64>,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            small_blind: 50,
            big_blind: 100,
            starting_chips: 10000,
            max_players: 9,
            min_players: 2,
            allow_rebuy: true,
            rebuy_amount: None,
        }
    }
}

/// Texas Hold'em poker engine.
///
/// Manages game state, betting rounds, pot distribution, and player actions.
/// Card values are not known to the engine — it communicates with an external
/// dealer via [`GameCommand`] / [`GameEvent`] messages.
///
/// # Game flow
///
/// ```text
/// add_player() → start_hand() → player_action() → ... → GameOver → start_hand()
///                  │                   │
///                  ▼                   ▼
///            [GameCommand]       [GameCommand]
///            [GameEvent]         [GameEvent]
/// ```
///
/// # Examples
///
/// ```rust
/// use poker_engine::{Game, GameConfig, PlayerAction, GameEvent};
///
/// let mut game = Game::new(GameConfig::default());
/// game.add_player(1).unwrap();
/// game.add_player(2).unwrap();
///
/// // Start hand — get commands for dealer
/// let cmds = game.start_hand().unwrap();
///
/// // Feed dealer results back
/// game.handle_event(GameEvent::HoleCardsDealt { player_id: 1 }).unwrap();
/// game.handle_event(GameEvent::HoleCardsDealt { player_id: 2 }).unwrap();
///
/// // Player actions
/// let active = game.active_player().unwrap();
/// game.player_action(active, PlayerAction::Call).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct Game {
    config: GameConfig,
    players: Vec<PlayerState>,
    phase: GamePhase,
    pot: Pot,
    current_bet: u64,
    dealer_index: usize,
    acting_index: Option<usize>,
    last_raiser_index: Option<usize>,
    has_acted: Vec<bool>,
    hand_number: u32,
    community_count: u8,
    pending_scores: Vec<(PlayerId, HandScore)>,
}

impl Game {
    /// Create a new game with the given configuration.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use poker_engine::{Game, GameConfig, GamePhase};
    ///
    /// let game = Game::new(GameConfig::default());
    /// assert_eq!(game.phase(), GamePhase::WaitingToStart);
    /// ```
    pub fn new(config: GameConfig) -> Self {
        Self {
            config,
            players: Vec::new(),
            phase: GamePhase::WaitingToStart,
            pot: Pot::default(),
            current_bet: 0,
            dealer_index: 0,
            acting_index: None,
            last_raiser_index: None,
            has_acted: Vec::new(),
            hand_number: 0,
            community_count: 0,
            pending_scores: Vec::new(),
        }
    }

    /// Add a player to the table. Can only be done between hands.
    ///
    /// The new player is inserted at a position in the seat order so that:
    /// - With 2 players (heads-up → 3): new player gets small blind.
    /// - With 3+ players: new player gets big blind.
    ///
    /// # Errors
    ///
    /// Returns [`PokerError::GameInProgress`] if a hand is in progress.
    /// Returns [`PokerError::PlayerAlreadyAtTable`] if the player is already seated.
    /// Returns [`PokerError::TableFull`] if the table is at capacity.
    pub fn add_player(&mut self, player_id: PlayerId) -> Result<(), PokerError> {
        if self.phase != GamePhase::WaitingToStart && self.phase != GamePhase::GameOver {
            return Err(PokerError::GameInProgress);
        }
        if self.players.iter().any(|p| p.id == player_id) {
            return Err(PokerError::PlayerAlreadyAtTable(player_id));
        }
        if self.players.len() >= self.config.max_players {
            return Err(PokerError::TableFull {
                max: self.config.max_players,
            });
        }

        let n = self.players.len();
        let new_player = PlayerState::new(player_id, self.config.starting_chips);

        if n < 2 {
            self.players.push(new_player);
        } else {
            let d = self.dealer_index;
            let insert_at = if n == 2 {
                d
            } else {
                let p = (d + 4) % (n + 1);
                if p <= d { p } else { (d + 3) % (n + 1) }
            };
            self.players.insert(insert_at, new_player);
            if insert_at <= d {
                self.dealer_index += 1;
            }
        }

        Ok(())
    }

    /// Remove a player from the table. Can only be done between hands.
    pub fn remove_player(&mut self, player_id: PlayerId) -> Result<(), PokerError> {
        if self.phase != GamePhase::WaitingToStart && self.phase != GamePhase::GameOver {
            return Err(PokerError::GameInProgress);
        }
        let idx = self
            .players
            .iter()
            .position(|p| p.id == player_id)
            .ok_or(PokerError::PlayerNotFound(player_id))?;
        self.players[idx].status = PlayerStatus::Out;
        Ok(())
    }

    /// Set a player to sit out. They will auto-fold each hand until sit_in is called.
    pub fn sit_out(&mut self, player_id: PlayerId) -> Result<(), PokerError> {
        let player = self
            .players
            .iter_mut()
            .find(|p| p.id == player_id)
            .ok_or(PokerError::PlayerNotFound(player_id))?;
        if player.status == PlayerStatus::Active {
            player.status = PlayerStatus::SittingOut;
            player.wants_in = false;
        }
        Ok(())
    }

    /// Set a player to sit back in. They will rejoin at the start of the next hand.
    pub fn sit_in(&mut self, player_id: PlayerId) -> Result<(), PokerError> {
        let player = self
            .players
            .iter_mut()
            .find(|p| p.id == player_id)
            .ok_or(PokerError::PlayerNotFound(player_id))?;
        if player.status == PlayerStatus::SittingOut {
            player.wants_in = true;
        }
        Ok(())
    }

    /// Add chips to a player's stack (rebuy).
    pub fn rebuy(&mut self, player_id: PlayerId, amount: u64) -> Result<(), PokerError> {
        if !self.config.allow_rebuy {
            return Err(PokerError::InvalidAction(
                "Rebuys are not allowed".to_string(),
            ));
        }
        let player = self
            .players
            .iter_mut()
            .find(|p| p.id == player_id)
            .ok_or(PokerError::PlayerNotFound(player_id))?;

        let max_amount = self.config.rebuy_amount.unwrap_or(self.config.starting_chips);
        if amount == 0 || player.chips + amount > max_amount {
            return Err(PokerError::InvalidRebuy {
                player: player_id,
                amount,
            });
        }

        player.chips += amount;
        if player.status == PlayerStatus::SittingOut && player.chips > 0 {
            player.wants_in = true;
        }
        Ok(())
    }

    /// Returns `true` if there are enough eligible players to start a hand.
    pub fn can_start_hand(&self) -> bool {
        let eligible = self
            .players
            .iter()
            .filter(|p| p.status == PlayerStatus::Active && p.chips > 0)
            .count();
        eligible >= self.config.min_players
    }

    /// Start a new hand. Posts blinds, deals cards, and returns commands for the dealer.
    ///
    /// The returned [`GameCommand`] list tells the dealer which cards to deal.
    /// After dealing, feed the results back via [`handle_event`](Self::handle_event).
    ///
    /// # Errors
    ///
    /// Returns [`PokerError::CannotStartHand`] if not enough eligible players.
    pub fn start_hand(&mut self) -> Result<Vec<GameCommand>, PokerError> {
        for player in &mut self.players {
            if player.status == PlayerStatus::SittingOut && player.wants_in {
                player.status = PlayerStatus::Active;
            }
            if player.status == PlayerStatus::Active && player.chips == 0 {
                if self.config.allow_rebuy {
                    player.status = PlayerStatus::SittingOut;
                    player.wants_in = true;
                } else {
                    player.status = PlayerStatus::SittingOut;
                    player.wants_in = false;
                }
            }
        }

        if !self.can_start_hand() {
            return Err(PokerError::CannotStartHand);
        }

        self.phase = GamePhase::PreFlop;
        self.hand_number += 1;
        self.pot = Pot::default();
        self.current_bet = 0;
        self.community_count = 0;
        self.last_raiser_index = None;
        self.acting_index = None;

        for player in &mut self.players {
            player.reset_for_new_hand();
        }

        for player in &mut self.players {
            player.is_dealer = false;
        }

        let eligible_indices: Vec<usize> = self
            .players
            .iter()
            .enumerate()
            .filter(|(_, p)| p.status == PlayerStatus::Active && p.chips > 0)
            .map(|(i, _)| i)
            .collect();

        if eligible_indices.len() < self.config.min_players {
            self.phase = GamePhase::GameOver;
            return Err(PokerError::CannotStartHand);
        }

        self.rotate_dealer(&eligible_indices);
        self.players[self.dealer_index].is_dealer = true;

        let dealer_pos = eligible_indices.iter().position(|&i| i == self.dealer_index).unwrap();
        let len = eligible_indices.len();
        let sb_index = eligible_indices[(dealer_pos + 1) % len];
        let bb_index = eligible_indices[(dealer_pos + 2) % len];

        let sb_amount = self.config.small_blind.min(self.players[sb_index].chips);
        let bb_amount = self.config.big_blind.min(self.players[bb_index].chips);

        self.players[sb_index].place_bet(sb_amount);
        self.players[bb_index].place_bet(bb_amount);

        self.pot.add_bet(self.players[sb_index].id, sb_amount);
        self.pot.add_bet(self.players[bb_index].id, bb_amount);

        self.current_bet = self.config.big_blind;

        self.has_acted = vec![false; self.players.len()];

        let mut commands = Vec::new();

        for &idx in &eligible_indices {
            if self.players[idx].status == PlayerStatus::Active && self.players[idx].chips > 0 {
                commands.push(GameCommand::DealHoleCards {
                    player_id: self.players[idx].id,
                });
            }
        }

        let first_to_act = if eligible_indices.len() == 2 {
            sb_index
        } else {
            let utg_offset = 3 % eligible_indices.len();
            eligible_indices[utg_offset]
        };

        self.acting_index = Some(first_to_act);

        Ok(commands)
    }

    /// Process a dealer event. Validates phase and updates game state.
    ///
    /// - `HoleCardsDealt` — valid only during PreFlop.
    /// - `CommunityCardsRevealed` — valid during Flop/Turn/River.
    /// - `PlayerCardsRevealed` — valid during Showdown. Collects scores and
    ///   determines the winner once all active players have been revealed.
    ///
    /// Returns additional commands if the engine needs more dealer actions.
    pub fn handle_event(&mut self, event: GameEvent) -> Result<Vec<GameCommand>, PokerError> {
        match event {
            GameEvent::HoleCardsDealt { player_id } => {
                if self.phase != GamePhase::PreFlop {
                    return Err(PokerError::InvalidAction(
                        "HoleCardsDealt only valid during PreFlop".to_string(),
                    ));
                }
                if !self.players.iter().any(|p| p.id == player_id) {
                    return Err(PokerError::PlayerNotFound(player_id));
                }
                Ok(Vec::new())
            }
            GameEvent::CommunityCardsRevealed => {
                match self.phase {
                    GamePhase::Flop | GamePhase::Turn | GamePhase::River => {}
                    _ => {
                        return Err(PokerError::InvalidAction(
                            "CommunityCardsRevealed only valid during Flop/Turn/River".to_string(),
                        ));
                    }
                }
                Ok(Vec::new())
            }
            GameEvent::PlayerCardsRevealed { player_id, score } => {
                if self.phase != GamePhase::Showdown {
                    return Err(PokerError::InvalidAction(
                        "PlayerCardsRevealed only valid during Showdown".to_string(),
                    ));
                }
                self.pending_scores.push((player_id, score));
                let active_count = self
                    .players
                    .iter()
                    .filter(|p| p.status == PlayerStatus::Active && p.is_active_in_hand())
                    .count();
                if self.pending_scores.len() >= active_count {
                    self.finish_showdown()
                } else {
                    Ok(Vec::new())
                }
            }
            GameEvent::Error(msg) => Err(PokerError::InvalidAction(msg)),
        }
    }

    /// Execute a player action (fold, check, call, raise, all-in).
    ///
    /// Returns commands if the action triggers a phase transition (e.g., revealing community cards).
    ///
    /// # Errors
    ///
    /// Returns [`PokerError::NotYourTurn`] if it is not this player's turn.
    /// Returns [`PokerError::PlayerIsAllIn`] if the player is all-in.
    pub fn player_action(
        &mut self,
        player_id: PlayerId,
        action: PlayerAction,
    ) -> Result<Vec<GameCommand>, PokerError> {
        if self.phase == GamePhase::WaitingToStart || self.phase == GamePhase::GameOver {
            return Err(PokerError::GameNotInProgress);
        }
        if self.phase == GamePhase::Showdown {
            return Err(PokerError::InvalidAction(
                "Game is in showdown phase".to_string(),
            ));
        }

        let acting = self.acting_index.ok_or(PokerError::InvalidAction(
            "No player acting".to_string(),
        ))?;

        if self.players[acting].id != player_id {
            return Err(PokerError::NotYourTurn {
                expected: self.players[acting].id,
                got: player_id,
            });
        }

        if !self.players[acting].can_act() {
            return Err(PokerError::PlayerIsAllIn(player_id));
        }

        match action {
            PlayerAction::Fold => {
                self.players[acting].status = PlayerStatus::SittingOut;
                self.players[acting].wants_in = true;
            }
            PlayerAction::Check => {
                if self.players[acting].bet < self.current_bet {
                    return Err(PokerError::InvalidAction(
                        "Cannot check, must call or raise".to_string(),
                    ));
                }
            }
            PlayerAction::Call => {
                let to_call = self.current_bet - self.players[acting].bet;
                let call_amount = to_call.min(self.players[acting].chips);
                self.players[acting].place_bet(call_amount);
                self.pot.add_bet(player_id, call_amount);
            }
            PlayerAction::Raise(amount) => {
                let total = self.current_bet + amount;
                if amount < self.config.big_blind {
                    return Err(PokerError::RaiseBelowMinimum {
                        minimum: self.config.big_blind,
                        attempted: amount,
                    });
                }
                let needed = total - self.players[acting].bet;
                if needed > self.players[acting].chips {
                    return Err(PokerError::NotEnoughChips {
                        player: player_id,
                        available: self.players[acting].chips,
                        required: needed,
                    });
                }
                self.players[acting].place_bet(needed);
                self.pot.add_bet(player_id, needed);
                self.current_bet = total;
                self.last_raiser_index = Some(acting);
                for acted in &mut self.has_acted {
                    *acted = false;
                }
            }
            PlayerAction::AllIn => {
                let all_in_amount = self.players[acting].chips;
                let total_after = self.players[acting].bet + all_in_amount;
                self.players[acting].place_bet(all_in_amount);
                self.pot.add_bet(player_id, all_in_amount);
                if total_after > self.current_bet {
                    self.current_bet = total_after;
                    self.last_raiser_index = Some(acting);
                    for acted in &mut self.has_acted {
                        *acted = false;
                    }
                }
            }
        }

        self.has_acted[acting] = true;
        self.advance_action()
    }

    fn advance_action(&mut self) -> Result<Vec<GameCommand>, PokerError> {
        let eligible_indices: Vec<usize> = self
            .players
            .iter()
            .enumerate()
            .filter(|(_, p)| p.status == PlayerStatus::Active && p.is_active_in_hand())
            .map(|(i, _)| i)
            .collect();

        let can_act: Vec<usize> = eligible_indices
            .iter()
            .filter(|&&i| self.players[i].can_act())
            .copied()
            .collect();

        if eligible_indices.len() <= 1 || can_act.is_empty() {
            return self.advance_to_next_phase();
        }

        if self.all_acting_done(&can_act) {
            return self.advance_to_next_phase();
        }

        let current = self.acting_index.unwrap();
        let mut next = current;
        loop {
            next = (next + 1) % self.players.len();
            if next == current {
                break;
            }
            if can_act.contains(&next) {
                self.acting_index = Some(next);
                return Ok(Vec::new());
            }
        }

        self.acting_index = Some(can_act[0]);
        Ok(Vec::new())
    }

    fn all_acting_done(&self, can_act: &[usize]) -> bool {
        if can_act.is_empty() {
            return true;
        }

        for &i in can_act {
            if !self.has_acted[i] {
                return false;
            }
        }

        can_act.iter().all(|&i| self.players[i].bet == self.current_bet)
    }

    fn advance_to_next_phase(&mut self) -> Result<Vec<GameCommand>, PokerError> {
        let remaining: Vec<usize> = self
            .players
            .iter()
            .enumerate()
            .filter(|(_, p)| p.status == PlayerStatus::Active && p.is_active_in_hand())
            .map(|(i, _)| i)
            .collect();

        if remaining.len() <= 1 {
            return self.finish_single_winner();
        }

        match self.phase {
            GamePhase::PreFlop => {
                self.reset_bets_for_new_round();
                self.phase = GamePhase::Flop;
                self.last_raiser_index = None;
                self.acting_index = self.find_first_to_act_after_dealer();
                Ok(vec![GameCommand::RevealCommunityCards { count: 3 }])
            }
            GamePhase::Flop => {
                self.community_count = 3;
                self.reset_bets_for_new_round();
                self.phase = GamePhase::Turn;
                self.last_raiser_index = None;
                self.acting_index = self.find_first_to_act_after_dealer();
                Ok(vec![GameCommand::RevealCommunityCards { count: 1 }])
            }
            GamePhase::Turn => {
                self.community_count = 4;
                self.reset_bets_for_new_round();
                self.phase = GamePhase::River;
                self.last_raiser_index = None;
                self.acting_index = self.find_first_to_act_after_dealer();
                Ok(vec![GameCommand::RevealCommunityCards { count: 1 }])
            }
            GamePhase::River => {
                self.community_count = 5;
                self.reset_bets_for_new_round();
                self.phase = GamePhase::Showdown;
                self.last_raiser_index = None;
                self.acting_index = None;

                let showdown_ids: Vec<PlayerId> = self
                    .players
                    .iter()
                    .filter(|p| p.status == PlayerStatus::Active && p.is_active_in_hand())
                    .map(|p| p.id)
                    .collect();

                if showdown_ids.len() <= 1 {
                    return self.finish_single_winner();
                }

                Ok(vec![GameCommand::RevealPlayerCards {
                    player_ids: showdown_ids,
                }])
            }
            GamePhase::Showdown => self.finish_showdown(),
            GamePhase::GameOver | GamePhase::WaitingToStart => Ok(Vec::new()),
        }
    }

    fn finish_single_winner(&mut self) -> Result<Vec<GameCommand>, PokerError> {
        let winner = self
            .players
            .iter()
            .find(|p| p.status == PlayerStatus::Active && p.is_active_in_hand());

        if let Some(winner) = winner {
            let total = self.pot.total();
            let winner_id = winner.id;
            self.players
                .iter_mut()
                .find(|p| p.id == winner_id)
                .unwrap()
                .collect_winnings(total);
        }

        self.pot = Pot::default();
        self.phase = GamePhase::GameOver;
        Ok(Vec::new())
    }

    fn rotate_dealer(&mut self, eligible_indices: &[usize]) {
        if self.hand_number == 1 {
            self.dealer_index = eligible_indices[0];
        } else {
            let dealer_pos = eligible_indices
                .iter()
                .position(|&i| i > self.dealer_index)
                .unwrap_or(0);
            self.dealer_index = eligible_indices[dealer_pos % eligible_indices.len()];
        }
    }

    fn finish_showdown(&mut self) -> Result<Vec<GameCommand>, PokerError> {
        let scores = std::mem::take(&mut self.pending_scores);

        if scores.is_empty() {
            self.pot = Pot::default();
            self.phase = GamePhase::GameOver;
            return Ok(Vec::new());
        }

        let max_score = scores.iter().map(|&(_, s)| s).max().unwrap();
        let winners: Vec<PlayerId> = scores
            .iter()
            .filter(|&&(_, s)| s == max_score)
            .map(|&(id, _)| id)
            .collect();

        let payouts = self.pot.distribute(&winners);

        for (player_id, amount) in payouts {
            if let Some(player) = self.players.iter_mut().find(|p| p.id == player_id) {
                player.collect_winnings(amount);
            }
        }

        self.pot = Pot::default();
        self.phase = GamePhase::GameOver;
        Ok(Vec::new())
    }

    fn reset_bets_for_new_round(&mut self) {
        for player in &mut self.players {
            player.bet = 0;
        }
        self.current_bet = 0;
        for acted in &mut self.has_acted {
            *acted = false;
        }
    }

    fn find_first_to_act_after_dealer(&self) -> Option<usize> {
        let eligible: Vec<usize> = self
            .players
            .iter()
            .enumerate()
            .filter(|(_, p)| p.status == PlayerStatus::Active && p.can_act())
            .map(|(i, _)| i)
            .collect();

        if eligible.is_empty() {
            return None;
        }

        eligible
            .iter()
            .find(|&&i| i > self.dealer_index)
            .copied()
            .or_else(|| eligible.first().copied())
    }

    /// Current game phase.
    pub fn phase(&self) -> GamePhase {
        self.phase
    }

    /// ID of the player who should act next, or None if no one is acting.
    pub fn active_player(&self) -> Option<PlayerId> {
        self.acting_index.map(|i| self.players[i].id)
    }

    /// List of players still at the table (Active or SittingOut).
    pub fn players(&self) -> Vec<&PlayerState> {
        self.players
            .iter()
            .filter(|p| p.status.is_in_game())
            .collect()
    }

    /// All players including those who have left.
    pub fn all_players(&self) -> &[PlayerState] {
        &self.players
    }

    /// Total amount in the pot.
    pub fn pot_total(&self) -> u64 {
        self.pot.total()
    }

    /// Current hand number (starts at 1).
    pub fn hand_number(&self) -> u32 {
        self.hand_number
    }

    /// Game configuration reference.
    pub fn config(&self) -> &GameConfig {
        &self.config
    }

    /// Current bet to match (big blind or last raise).
    pub fn current_bet(&self) -> u64 {
        self.current_bet
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> GameConfig {
        GameConfig {
            small_blind: 50,
            big_blind: 100,
            starting_chips: 1000,
            max_players: 9,
            min_players: 2,
            allow_rebuy: true,
            rebuy_amount: None,
        }
    }

    fn setup_two_player_game() -> Game {
        let mut game = Game::new(default_config());
        game.add_player(1).unwrap();
        game.add_player(2).unwrap();
        game
    }

    fn setup_three_player_game() -> Game {
        let mut game = Game::new(default_config());
        game.add_player(1).unwrap();
        game.add_player(2).unwrap();
        game.add_player(3).unwrap();
        game
    }

    // === Player management tests ===

    #[test]
    fn test_add_player() {
        let mut game = Game::new(default_config());
        assert!(game.add_player(1).is_ok());
        assert_eq!(game.players().len(), 1);
    }

    #[test]
    fn test_add_duplicate_player() {
        let mut game = Game::new(default_config());
        game.add_player(1).unwrap();
        assert_eq!(game.add_player(1), Err(PokerError::PlayerAlreadyAtTable(1)));
    }

    #[test]
    fn test_table_full() {
        let config = GameConfig {
            max_players: 2,
            ..default_config()
        };
        let mut game = Game::new(config);
        game.add_player(1).unwrap();
        game.add_player(2).unwrap();
        assert_eq!(
            game.add_player(3),
            Err(PokerError::TableFull { max: 2 })
        );
    }

    #[test]
    fn test_add_player_during_game() {
        let mut game = setup_two_player_game();
        game.start_hand().unwrap();
        assert_eq!(game.add_player(3), Err(PokerError::GameInProgress));
    }

    #[test]
    fn test_remove_player() {
        let mut game = Game::new(default_config());
        game.add_player(1).unwrap();
        game.add_player(2).unwrap();
        assert!(game.remove_player(1).is_ok());
        assert_eq!(game.all_players()[0].status, PlayerStatus::Out);
    }

    #[test]
    fn test_remove_nonexistent_player() {
        let mut game = Game::new(default_config());
        assert_eq!(game.remove_player(99), Err(PokerError::PlayerNotFound(99)));
    }

    #[test]
    fn test_sit_out_and_sit_in() {
        let mut game = setup_two_player_game();
        game.sit_out(1).unwrap();
        assert_eq!(game.all_players()[0].status, PlayerStatus::SittingOut);
        assert!(!game.all_players()[0].wants_in);
        game.sit_in(1).unwrap();
        assert!(game.all_players()[0].wants_in);
    }

    #[test]
    fn test_rebuy() {
        let config = GameConfig {
            starting_chips: 2000,
            rebuy_amount: Some(3000),
            ..default_config()
        };
        let mut game = Game::new(config);
        game.add_player(1).unwrap();
        game.add_player(2).unwrap();
        let p = game.all_players().iter().find(|p| p.id == 1).unwrap();
        assert_eq!(p.chips, 2000);
        game.rebuy(1, 500).unwrap();
        let p = game.all_players().iter().find(|p| p.id == 1).unwrap();
        assert_eq!(p.chips, 2500);
    }

    #[test]
    fn test_rebuy_disabled() {
        let config = GameConfig {
            allow_rebuy: false,
            ..default_config()
        };
        let mut game = Game::new(config);
        game.add_player(1).unwrap();
        assert!(game.rebuy(1, 500).is_err());
    }

    #[test]
    fn test_rebuy_exceeds_max() {
        let config = GameConfig {
            rebuy_amount: Some(1200),
            ..default_config()
        };
        let mut game = Game::new(config);
        game.add_player(1).unwrap();
        assert!(game.rebuy(1, 1500).is_err());
    }

    // === Hand start tests ===

    #[test]
    fn test_can_start_hand() {
        let mut game = Game::new(default_config());
        assert!(!game.can_start_hand());
        game.add_player(1).unwrap();
        assert!(!game.can_start_hand());
        game.add_player(2).unwrap();
        assert!(game.can_start_hand());
    }

    #[test]
    fn test_start_hand_commands() {
        let mut game = setup_two_player_game();
        let cmds = game.start_hand().unwrap();
        assert_eq!(cmds.len(), 2);
        assert!(cmds.contains(&GameCommand::DealHoleCards { player_id: 1 }));
        assert!(cmds.contains(&GameCommand::DealHoleCards { player_id: 2 }));
        assert_eq!(game.phase(), GamePhase::PreFlop);
        assert_eq!(game.hand_number(), 1);
    }

    #[test]
    fn test_blinds_posted() {
        let mut game = setup_two_player_game();
        game.start_hand().unwrap();
        let p1 = game.all_players().iter().find(|p| p.id == 1).unwrap();
        let p2 = game.all_players().iter().find(|p| p.id == 2).unwrap();
        let sb = p1.bet + p2.bet;
        assert_eq!(sb, 150);
    }

    #[test]
    fn test_start_hand_not_enough_players() {
        let mut game = Game::new(default_config());
        game.add_player(1).unwrap();
        assert_eq!(game.start_hand(), Err(PokerError::CannotStartHand));
    }

    #[test]
    fn test_dealer_rotation() {
        let mut game = setup_three_player_game();
        game.start_hand().unwrap();
        let dealer_id = game.all_players().iter().find(|p| p.is_dealer).unwrap().id;
        // With new insertion order [P3, P1, P2], hand 1 dealer is P3
        assert_eq!(dealer_id, 3);

        for _ in 0..3 {
            game.handle_event(GameEvent::HoleCardsDealt {
                player_id: game.active_player().unwrap(),
            })
            .unwrap();
        }

        while game.phase() == GamePhase::PreFlop {
            let active = game.active_player().unwrap();
            game.player_action(active, PlayerAction::Call).unwrap();
        }

        while game.phase() == GamePhase::Flop {
            let cmds = game
                .handle_event(GameEvent::CommunityCardsRevealed)
                .unwrap();
            if cmds.is_empty() {
                let active = game.active_player().unwrap();
                game.player_action(active, PlayerAction::Check).unwrap();
            }
        }

        while game.phase() == GamePhase::Turn {
            let cmds = game
                .handle_event(GameEvent::CommunityCardsRevealed)
                .unwrap();
            if cmds.is_empty() {
                let active = game.active_player().unwrap();
                game.player_action(active, PlayerAction::Check).unwrap();
            }
        }

        while game.phase() == GamePhase::River {
            let cmds = game
                .handle_event(GameEvent::CommunityCardsRevealed)
                .unwrap();
            if cmds.is_empty() {
                let active = game.active_player().unwrap();
                game.player_action(active, PlayerAction::Check).unwrap();
            }
        }

        if game.phase() == GamePhase::Showdown {
            let ids: Vec<PlayerId> = game
                .all_players()
                .iter()
                .filter(|p| p.status == PlayerStatus::Active && p.is_active_in_hand())
                .map(|p| p.id)
                .collect();
            for id in ids {
                let _ = game.handle_event(GameEvent::PlayerCardsRevealed {
                    player_id: id,
                    score: 100,
                });
            }
        }

        game.start_hand().unwrap();
        let dealer_id = game.all_players().iter().find(|p| p.is_dealer).unwrap().id;
        // Hand 2: rotation from P3 → next is P1
        assert_eq!(dealer_id, 1);
    }

    // === Betting action tests ===

    #[test]
    fn test_fold() {
        let mut game = setup_two_player_game();
        game.start_hand().unwrap();
        game.handle_event(GameEvent::HoleCardsDealt {
            player_id: 1,
        })
        .unwrap();
        game.handle_event(GameEvent::HoleCardsDealt {
            player_id: 2,
        })
        .unwrap();

        let active = game.active_player().unwrap();
        game.player_action(active, PlayerAction::Fold).unwrap();
        assert_eq!(game.phase(), GamePhase::GameOver);
    }

    #[test]
    fn test_check_preflop() {
        let mut game = setup_two_player_game();
        game.start_hand().unwrap();
        game.handle_event(GameEvent::HoleCardsDealt {
            player_id: 1,
        })
        .unwrap();
        game.handle_event(GameEvent::HoleCardsDealt {
            player_id: 2,
        })
        .unwrap();

        let active = game.active_player().unwrap();
        assert!(game.player_action(active, PlayerAction::Check).is_err());
    }

    #[test]
    fn test_call() {
        let mut game = setup_two_player_game();
        game.start_hand().unwrap();
        game.handle_event(GameEvent::HoleCardsDealt {
            player_id: 1,
        })
        .unwrap();
        game.handle_event(GameEvent::HoleCardsDealt {
            player_id: 2,
        })
        .unwrap();

        let active = game.active_player().unwrap();
        game.player_action(active, PlayerAction::Call).unwrap();
        let p = game
            .all_players()
            .iter()
            .find(|p| p.id == active)
            .unwrap();
        assert_eq!(p.bet, 100);
    }

    #[test]
    fn test_raise() {
        let mut game = setup_two_player_game();
        game.start_hand().unwrap();
        game.handle_event(GameEvent::HoleCardsDealt {
            player_id: 1,
        })
        .unwrap();
        game.handle_event(GameEvent::HoleCardsDealt {
            player_id: 2,
        })
        .unwrap();

        let active = game.active_player().unwrap();
        game.player_action(active, PlayerAction::Raise(100)).unwrap();
        assert_eq!(game.current_bet(), 200);
    }

    #[test]
    fn test_all_in() {
        let mut game = setup_two_player_game();
        game.start_hand().unwrap();
        game.handle_event(GameEvent::HoleCardsDealt {
            player_id: 1,
        })
        .unwrap();
        game.handle_event(GameEvent::HoleCardsDealt {
            player_id: 2,
        })
        .unwrap();

        let active = game.active_player().unwrap();
        game.player_action(active, PlayerAction::AllIn).unwrap();
        let p = game
            .all_players()
            .iter()
            .find(|p| p.id == active)
            .unwrap();
        assert!(p.all_in);
        assert_eq!(p.chips, 0);
    }

    #[test]
    fn test_wrong_player_turn() {
        let mut game = setup_two_player_game();
        game.start_hand().unwrap();
        game.handle_event(GameEvent::HoleCardsDealt {
            player_id: 1,
        })
        .unwrap();
        game.handle_event(GameEvent::HoleCardsDealt {
            player_id: 2,
        })
        .unwrap();

        let active = game.active_player().unwrap();
        let other = if active == 1 { 2 } else { 1 };
        assert!(game.player_action(other, PlayerAction::Check).is_err());
    }

    #[test]
    fn test_cannot_raise_below_minimum() {
        let mut game = setup_two_player_game();
        game.start_hand().unwrap();
        game.handle_event(GameEvent::HoleCardsDealt {
            player_id: 1,
        })
        .unwrap();
        game.handle_event(GameEvent::HoleCardsDealt {
            player_id: 2,
        })
        .unwrap();

        let active = game.active_player().unwrap();
        assert!(game.player_action(active, PlayerAction::Raise(10)).is_err());
    }

    // === Phase transition tests ===

    #[test]
    fn test_preflop_to_flop() {
        let mut game = setup_two_player_game();
        game.start_hand().unwrap();
        game.handle_event(GameEvent::HoleCardsDealt {
            player_id: 1,
        })
        .unwrap();
        game.handle_event(GameEvent::HoleCardsDealt {
            player_id: 2,
        })
        .unwrap();

        let active = game.active_player().unwrap();
        game.player_action(active, PlayerAction::Call).unwrap();
        let other = if active == 1 { 2 } else { 1 };
        game.player_action(other, PlayerAction::Check).unwrap();

        assert_eq!(game.phase(), GamePhase::Flop);
    }

    #[test]
    fn test_full_hand_checkdown() {
        let mut game = setup_two_player_game();
        game.start_hand().unwrap();
        game.handle_event(GameEvent::HoleCardsDealt {
            player_id: 1,
        })
        .unwrap();
        game.handle_event(GameEvent::HoleCardsDealt {
            player_id: 2,
        })
        .unwrap();

        let active = game.active_player().unwrap();
        let other = if active == 1 { 2 } else { 1 };

        game.player_action(active, PlayerAction::Call).unwrap();
        game.player_action(other, PlayerAction::Check).unwrap();
        assert_eq!(game.phase(), GamePhase::Flop);

        game.handle_event(GameEvent::CommunityCardsRevealed)
        .unwrap();
        assert_eq!(game.phase(), GamePhase::Flop);

        let a2 = game.active_player().unwrap();
        let o2 = if a2 == 1 { 2 } else { 1 };
        game.player_action(a2, PlayerAction::Check).unwrap();
        game.player_action(o2, PlayerAction::Check).unwrap();
        assert_eq!(game.phase(), GamePhase::Turn);

        game.handle_event(GameEvent::CommunityCardsRevealed)
        .unwrap();
        assert_eq!(game.phase(), GamePhase::Turn);

        let a3 = game.active_player().unwrap();
        let o3 = if a3 == 1 { 2 } else { 1 };
        game.player_action(a3, PlayerAction::Check).unwrap();
        game.player_action(o3, PlayerAction::Check).unwrap();
        assert_eq!(game.phase(), GamePhase::River);

        game.handle_event(GameEvent::CommunityCardsRevealed)
        .unwrap();
        assert_eq!(game.phase(), GamePhase::River);

        let a4 = game.active_player().unwrap();
        let o4 = if a4 == 1 { 2 } else { 1 };
        game.player_action(a4, PlayerAction::Check).unwrap();
        game.player_action(o4, PlayerAction::Check).unwrap();
        assert_eq!(game.phase(), GamePhase::Showdown);
    }

    #[test]
    fn test_showdown_with_winner() {
        let mut game = setup_two_player_game();
        game.start_hand().unwrap();
        game.handle_event(GameEvent::HoleCardsDealt {
            player_id: 1,
        })
        .unwrap();
        game.handle_event(GameEvent::HoleCardsDealt {
            player_id: 2,
        })
        .unwrap();

        let active = game.active_player().unwrap();
        let other = if active == 1 { 2 } else { 1 };

        game.player_action(active, PlayerAction::Call).unwrap();
        game.player_action(other, PlayerAction::Check).unwrap();

        game.handle_event(GameEvent::CommunityCardsRevealed)
        .unwrap();
        let a2 = game.active_player().unwrap();
        let o2 = if a2 == 1 { 2 } else { 1 };
        game.player_action(a2, PlayerAction::Check).unwrap();
        game.player_action(o2, PlayerAction::Check).unwrap();

        game.handle_event(GameEvent::CommunityCardsRevealed)
        .unwrap();
        let a3 = game.active_player().unwrap();
        let o3 = if a3 == 1 { 2 } else { 1 };
        game.player_action(a3, PlayerAction::Check).unwrap();
        game.player_action(o3, PlayerAction::Check).unwrap();

        game.handle_event(GameEvent::CommunityCardsRevealed)
        .unwrap();

        let a4 = game.active_player().unwrap();
        let o4 = if a4 == 1 { 2 } else { 1 };
        game.player_action(a4, PlayerAction::Check).unwrap();
        game.player_action(o4, PlayerAction::Check).unwrap();
        assert_eq!(game.phase(), GamePhase::Showdown);

        let _ = game.handle_event(GameEvent::PlayerCardsRevealed {
            player_id: 1,
            score: 100,
        });
        let _ = game.handle_event(GameEvent::PlayerCardsRevealed {
            player_id: 2,
            score: 200,
        });

        assert_eq!(game.phase(), GamePhase::GameOver);
        let winner = game
            .all_players()
            .iter()
            .find(|p| p.id == 2)
            .unwrap();
        assert!(winner.chips > 1000);
    }

    // === Single winner (everyone else folds) ===

    #[test]
    fn test_fold_gives_pot_to_remaining_player() {
        let mut game = setup_two_player_game();
        let p1_start = game.all_players().iter().find(|p| p.id == 1).unwrap().chips;
        let p2_start = game.all_players().iter().find(|p| p.id == 2).unwrap().chips;

        game.start_hand().unwrap();
        game.handle_event(GameEvent::HoleCardsDealt {
            player_id: 1,
        })
        .unwrap();
        game.handle_event(GameEvent::HoleCardsDealt {
            player_id: 2,
        })
        .unwrap();

        let active = game.active_player().unwrap();
        let other = if active == 1 { 2 } else { 1 };
        game.player_action(active, PlayerAction::Fold).unwrap();

        assert_eq!(game.phase(), GamePhase::GameOver);
        let _winner = game.all_players().iter().find(|p| p.id == other).unwrap();
        let total_chips: u64 = game
            .all_players()
            .iter()
            .map(|p| p.chips)
            .sum();
        assert_eq!(total_chips, p1_start + p2_start);
    }

    // === Game not in progress ===

    #[test]
    fn test_player_action_when_not_started() {
        let mut game = setup_two_player_game();
        assert!(game.player_action(1, PlayerAction::Check).is_err());
    }

    #[test]
    fn test_player_action_after_game_over() {
        let mut game = setup_two_player_game();
        game.start_hand().unwrap();
        game.handle_event(GameEvent::HoleCardsDealt {
            player_id: 1,
        })
        .unwrap();
        game.handle_event(GameEvent::HoleCardsDealt {
            player_id: 2,
        })
        .unwrap();
        let active = game.active_player().unwrap();
        game.player_action(active, PlayerAction::Fold).unwrap();
        assert_eq!(game.phase(), GamePhase::GameOver);
        assert!(game.player_action(1, PlayerAction::Check).is_err());
    }

    // === Multiple hands ===

    #[test]
    fn test_multiple_hands() {
        let mut game = setup_two_player_game();

        for i in 0..3 {
            game.start_hand().unwrap();
            assert_eq!(game.hand_number(), i + 1);
            game.handle_event(GameEvent::HoleCardsDealt {
                player_id: 1,
            })
            .unwrap();
            game.handle_event(GameEvent::HoleCardsDealt {
                player_id: 2,
            })
            .unwrap();
            let active = game.active_player().unwrap();
            game.player_action(active, PlayerAction::Fold).unwrap();
            assert_eq!(game.phase(), GamePhase::GameOver);
        }
        assert_eq!(game.hand_number(), 3);
    }

    // === Config tests ===

    #[test]
    fn test_config_custom() {
        let config = GameConfig {
            small_blind: 100,
            big_blind: 200,
            starting_chips: 5000,
            max_players: 6,
            min_players: 3,
            allow_rebuy: false,
            rebuy_amount: Some(3000),
        };
        let mut game = Game::new(config);
        game.add_player(1).unwrap();
        game.add_player(2).unwrap();
        game.add_player(3).unwrap();
        assert_eq!(game.config().big_blind, 200);
        assert_eq!(game.config().max_players, 6);
    }

    // === Pot and chip tracking ===

    #[test]
    fn test_pot_tracks_bets() {
        let mut game = setup_two_player_game();
        game.start_hand().unwrap();
        game.handle_event(GameEvent::HoleCardsDealt {
            player_id: 1,
        })
        .unwrap();
        game.handle_event(GameEvent::HoleCardsDealt {
            player_id: 2,
        })
        .unwrap();

        assert!(game.pot_total() > 0);

        let active = game.active_player().unwrap();
        game.player_action(active, PlayerAction::Call).unwrap();
        let other = if active == 1 { 2 } else { 1 };
        game.player_action(other, PlayerAction::Check).unwrap();

        assert!(game.pot_total() > 150);
    }

    // === Event handling ===

    #[test]
    fn test_event_error() {
        let mut game = setup_two_player_game();
        let result = game.handle_event(GameEvent::Error("test".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_hole_cards_dealt_event() {
        let mut game = setup_two_player_game();
        game.start_hand().unwrap();
        game.handle_event(GameEvent::HoleCardsDealt {
            player_id: 1,
        })
        .unwrap();
        let p = game.all_players().iter().find(|p| p.id == 1).unwrap();
        assert!(p.is_active_in_hand());
    }

    // === Edge case: 3 player fold to one ===

    #[test]
    fn test_three_player_two_fold() {
        let mut game = setup_three_player_game();
        game.start_hand().unwrap();
        game.handle_event(GameEvent::HoleCardsDealt {
            player_id: 1,
        })
        .unwrap();
        game.handle_event(GameEvent::HoleCardsDealt {
            player_id: 2,
        })
        .unwrap();
        game.handle_event(GameEvent::HoleCardsDealt {
            player_id: 3,
        })
        .unwrap();

        let active = game.active_player().unwrap();
        game.player_action(active, PlayerAction::Fold).unwrap();

        let active2 = game.active_player().unwrap();
        game.player_action(active2, PlayerAction::Fold).unwrap();

        assert_eq!(game.phase(), GamePhase::GameOver);
    }

    // === All-in without raise ===

    #[test]
    fn test_all_in_less_than_current_bet() {
        let mut game = setup_two_player_game();
        game.start_hand().unwrap();
        game.handle_event(GameEvent::HoleCardsDealt {
            player_id: 1,
        })
        .unwrap();
        game.handle_event(GameEvent::HoleCardsDealt {
            player_id: 2,
        })
        .unwrap();

        let active = game.active_player().unwrap();
        game.player_action(active, PlayerAction::AllIn).unwrap();
        let other = if active == 1 { 2 } else { 1 };
        game.player_action(other, PlayerAction::Call).unwrap();

        assert_eq!(game.phase(), GamePhase::Flop);
    }

    // === New player position ===

    #[test]
    fn test_new_player_sb_in_heads_up() {
        let mut game = setup_two_player_game();

        // Play first hand to completion
        game.start_hand().unwrap();
        game.handle_event(GameEvent::HoleCardsDealt { player_id: 1 }).unwrap();
        game.handle_event(GameEvent::HoleCardsDealt { player_id: 2 }).unwrap();
        let active = game.active_player().unwrap();
        game.player_action(active, PlayerAction::Fold).unwrap();
        assert_eq!(game.phase(), GamePhase::GameOver);

        // Add third player — n=2→3, new player gets SB
        game.add_player(3).unwrap();

        game.start_hand().unwrap();

        let p3 = game.all_players().iter().find(|p| p.id == 3).unwrap();
        assert_eq!(p3.bet, game.config().small_blind, "player 3 should post small blind");

        let bb = game
            .all_players()
            .iter()
            .find(|p| p.bet == game.config().big_blind)
            .unwrap();
        assert_ne!(bb.id, 3, "player 3 should not be BB");
    }

    #[test]
    fn test_new_player_bb_with_three_players() {
        let mut game = setup_three_player_game();

        // Play first hand to completion
        game.start_hand().unwrap();
        game.handle_event(GameEvent::HoleCardsDealt { player_id: 1 }).unwrap();
        game.handle_event(GameEvent::HoleCardsDealt { player_id: 2 }).unwrap();
        game.handle_event(GameEvent::HoleCardsDealt { player_id: 3 }).unwrap();
        let active = game.active_player().unwrap();
        game.player_action(active, PlayerAction::Fold).unwrap();
        let active2 = game.active_player().unwrap();
        game.player_action(active2, PlayerAction::Fold).unwrap();
        assert_eq!(game.phase(), GamePhase::GameOver);

        // Add fourth player — n=3→4, new player gets BB
        game.add_player(4).unwrap();

        game.start_hand().unwrap();

        let p4 = game.all_players().iter().find(|p| p.id == 4).unwrap();
        assert_eq!(p4.bet, game.config().big_blind, "player 4 should post big blind");
    }

    #[test]
    fn test_normal_rotation_after_new_player() {
        let mut game = setup_two_player_game();

        // Play hand 1
        game.start_hand().unwrap();
        game.handle_event(GameEvent::HoleCardsDealt { player_id: 1 }).unwrap();
        game.handle_event(GameEvent::HoleCardsDealt { player_id: 2 }).unwrap();
        let active = game.active_player().unwrap();
        game.player_action(active, PlayerAction::Fold).unwrap();

        // Add player 3 — gets SB
        game.add_player(3).unwrap();

        // Hand 2 — player 3 is SB
        game.start_hand().unwrap();
        let sb = game
            .all_players()
            .iter()
            .find(|p| p.bet == game.config().small_blind)
            .unwrap();
        assert_eq!(sb.id, 3, "player 3 should be SB");

        // Play to completion
        game.handle_event(GameEvent::HoleCardsDealt { player_id: 1 }).unwrap();
        game.handle_event(GameEvent::HoleCardsDealt { player_id: 2 }).unwrap();
        game.handle_event(GameEvent::HoleCardsDealt { player_id: 3 }).unwrap();
        let a = game.active_player().unwrap();
        game.player_action(a, PlayerAction::Fold).unwrap();
        let a2 = game.active_player().unwrap();
        game.player_action(a2, PlayerAction::Fold).unwrap();
        assert_eq!(game.phase(), GamePhase::GameOver);

        // Hand 3 — normal rotation
        game.start_hand().unwrap();
        let total_bets: u64 = game.all_players().iter().map(|p| p.bet).sum();
        assert_eq!(total_bets, game.config().small_blind + game.config().big_blind);
    }
}
