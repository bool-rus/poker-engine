use crate::error::PlayerId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerStatus {
    Active,
    SittingOut,
    Out,
}

impl PlayerStatus {
    pub fn is_in_game(&self) -> bool {
        matches!(self, PlayerStatus::Active | PlayerStatus::SittingOut)
    }
}

#[derive(Debug, Clone)]
pub struct PlayerState {
    pub id: PlayerId,
    pub chips: u64,
    pub bet: u64,
    pub status: PlayerStatus,
    pub is_dealer: bool,
    pub wants_in: bool,
    pub all_in: bool,
    pub hand_score: Option<u64>,
}

impl PlayerState {
    pub fn new(id: PlayerId, chips: u64) -> Self {
        Self {
            id,
            chips,
            bet: 0,
            status: PlayerStatus::Active,
            is_dealer: false,
            wants_in: true,
            all_in: false,
            hand_score: None,
        }
    }

    pub fn is_active_in_hand(&self) -> bool {
        self.status == PlayerStatus::Active && (self.chips > 0 || self.bet > 0)
    }

    pub fn can_act(&self) -> bool {
        self.status == PlayerStatus::Active && !self.all_in && self.chips > 0
    }

    pub fn place_bet(&mut self, amount: u64) {
        let actual = amount.min(self.chips);
        self.chips -= actual;
        self.bet += actual;
        if self.chips == 0 {
            self.all_in = true;
        }
    }

    pub fn collect_winnings(&mut self, amount: u64) {
        self.chips += amount;
    }

    pub fn reset_for_new_hand(&mut self) {
        self.bet = 0;
        self.all_in = false;
        self.hand_score = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_player() {
        let p = PlayerState::new(1, 1000);
        assert_eq!(p.id, 1);
        assert_eq!(p.chips, 1000);
        assert_eq!(p.bet, 0);
        assert_eq!(p.status, PlayerStatus::Active);
        assert!(!p.is_dealer);
        assert!(p.wants_in);
        assert!(!p.all_in);
        assert!(p.hand_score.is_none());
    }

    #[test]
    fn test_is_active_in_hand() {
        let mut p = PlayerState::new(1, 100);
        assert!(p.is_active_in_hand());
        p.status = PlayerStatus::SittingOut;
        assert!(!p.is_active_in_hand());
        p.status = PlayerStatus::Active;
        p.chips = 0;
        p.bet = 50;
        assert!(p.is_active_in_hand());
        p.bet = 0;
        assert!(!p.is_active_in_hand());
    }

    #[test]
    fn test_can_act() {
        let mut p = PlayerState::new(1, 100);
        assert!(p.can_act());
        p.all_in = true;
        assert!(!p.can_act());
        p.all_in = false;
        p.chips = 0;
        assert!(!p.can_act());
        p.chips = 50;
        p.status = PlayerStatus::SittingOut;
        assert!(!p.can_act());
    }

    #[test]
    fn test_place_bet_normal() {
        let mut p = PlayerState::new(1, 1000);
        p.place_bet(300);
        assert_eq!(p.chips, 700);
        assert_eq!(p.bet, 300);
        assert!(!p.all_in);
    }

    #[test]
    fn test_place_bet_all_in() {
        let mut p = PlayerState::new(1, 500);
        p.place_bet(500);
        assert_eq!(p.chips, 0);
        assert_eq!(p.bet, 500);
        assert!(p.all_in);
    }

    #[test]
    fn test_place_bet_clamps_to_chips() {
        let mut p = PlayerState::new(1, 200);
        p.place_bet(500);
        assert_eq!(p.chips, 0);
        assert_eq!(p.bet, 200);
        assert!(p.all_in);
    }

    #[test]
    fn test_collect_winnings() {
        let mut p = PlayerState::new(1, 500);
        p.collect_winnings(300);
        assert_eq!(p.chips, 800);
    }

    #[test]
    fn test_reset_for_new_hand() {
        let mut p = PlayerState::new(1, 1000);
        p.bet = 200;
        p.all_in = true;
        p.hand_score = Some(999);
        p.reset_for_new_hand();
        assert_eq!(p.bet, 0);
        assert!(!p.all_in);
        assert!(p.hand_score.is_none());
    }

    #[test]
    fn test_player_status_is_in_game() {
        assert!(PlayerStatus::Active.is_in_game());
        assert!(PlayerStatus::SittingOut.is_in_game());
        assert!(!PlayerStatus::Out.is_in_game());
    }
}
