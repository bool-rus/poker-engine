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
                let all_all_in = self.players.iter()
                    .filter(|p| p.status == PlayerStatus::Active && p.is_active_in_hand())
                    .all(|p| p.all_in);
                if all_all_in {
                    self.advance_to_next_phase()
                } else {
                    Ok(Vec::new())
                }
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
mod tests;
