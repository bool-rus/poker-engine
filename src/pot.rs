use std::collections::HashSet;
use crate::error::PlayerId;

#[derive(Debug, Clone)]
pub struct PotLayer {
    pub amount: u64,
    pub eligible: HashSet<PlayerId>,
}

#[derive(Debug, Clone, Default)]
pub struct Pot {
    pub layers: Vec<PotLayer>,
}

impl Pot {
    pub fn total(&self) -> u64 {
        self.layers.iter().map(|l| l.amount).sum()
    }

    pub fn add_bet(&mut self, player_id: PlayerId, amount: u64) {
        if self.layers.is_empty() {
            self.layers.push(PotLayer {
                amount,
                eligible: {
                    let mut set = HashSet::new();
                    set.insert(player_id);
                    set
                },
            });
        } else {
            self.layers.last_mut().unwrap().amount += amount;
            self.layers.last_mut().unwrap().eligible.insert(player_id);
        }
    }

    pub fn collect_from_bets(&mut self, bets: &[(PlayerId, u64)]) {
        let mut sorted: Vec<(PlayerId, u64)> = bets
            .iter()
            .filter(|(_, amount)| *amount > 0)
            .copied()
            .collect();
        sorted.sort_by_key(|&(_, amount)| amount);

        if sorted.is_empty() {
            return;
        }

        let mut layers = Vec::new();
        let mut prev_amount = 0u64;

        for &(_id, amount) in &sorted {
            let contribution = amount - prev_amount;
            if contribution > 0 {
                let eligible: HashSet<PlayerId> = sorted
                    .iter()
                    .filter(|&&(_, a)| a >= amount)
                    .map(|&(pid, _)| pid)
                    .collect();

                layers.push(PotLayer {
                    amount: contribution * eligible.len() as u64,
                    eligible,
                });
            }
            prev_amount = amount;
        }

        self.layers = layers;
    }

    pub fn distribute(&self, winners: &[PlayerId]) -> Vec<(PlayerId, u64)> {
        let mut payouts: Vec<(PlayerId, u64)> = Vec::new();

        for layer in &self.layers {
            if layer.amount == 0 || layer.eligible.is_empty() {
                continue;
            }

            let eligible_winners: Vec<PlayerId> = winners
                .iter()
                .filter(|id| layer.eligible.contains(id))
                .copied()
                .collect();

            if eligible_winners.is_empty() {
                let first_eligible = *layer.eligible.iter().next().unwrap();
                if let Some(entry) = payouts.iter_mut().find(|(id, _)| *id == first_eligible) {
                    entry.1 += layer.amount;
                } else {
                    payouts.push((first_eligible, layer.amount));
                }
            } else {
                let share = layer.amount / eligible_winners.len() as u64;
                let remainder = layer.amount % eligible_winners.len() as u64;

                for (i, &winner_id) in eligible_winners.iter().enumerate() {
                    let extra = if i == 0 { remainder } else { 0 };
                    if let Some(entry) = payouts.iter_mut().find(|(id, _)| *id == winner_id) {
                        entry.1 += share + extra;
                    } else {
                        payouts.push((winner_id, share + extra));
                    }
                }
            }
        }

        payouts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pot_total_empty() {
        let pot = Pot::default();
        assert_eq!(pot.total(), 0);
    }

    #[test]
    fn test_collect_from_bets_equal() {
        let mut pot = Pot::default();
        pot.collect_from_bets(&[(1, 100), (2, 100), (3, 100)]);
        assert_eq!(pot.total(), 300);
        assert_eq!(pot.layers.len(), 1);
        assert_eq!(pot.layers[0].amount, 300);
        assert!(pot.layers[0].eligible.contains(&1));
        assert!(pot.layers[0].eligible.contains(&2));
        assert!(pot.layers[0].eligible.contains(&3));
    }

    #[test]
    fn test_collect_from_bets_unequal() {
        let mut pot = Pot::default();
        pot.collect_from_bets(&[(1, 100), (2, 200), (3, 300)]);
        assert_eq!(pot.total(), 600);
        assert_eq!(pot.layers.len(), 3);
        assert_eq!(pot.layers[0].amount, 300);
        assert_eq!(pot.layers[1].amount, 200);
        assert_eq!(pot.layers[2].amount, 100);
    }

    #[test]
    fn test_collect_from_bets_with_zero() {
        let mut pot = Pot::default();
        pot.collect_from_bets(&[(1, 100), (2, 0), (3, 200)]);
        assert_eq!(pot.total(), 300);
    }

    #[test]
    fn test_collect_from_bets_empty() {
        let mut pot = Pot::default();
        pot.collect_from_bets(&[]);
        assert_eq!(pot.total(), 0);
    }

    #[test]
    fn test_distribute_single_winner() {
        let mut pot = Pot::default();
        pot.collect_from_bets(&[(1, 100), (2, 100), (3, 100)]);
        let payouts = pot.distribute(&[1]);
        assert_eq!(payouts.len(), 1);
        assert_eq!(payouts[0], (1, 300));
    }

    #[test]
    fn test_distribute_split_pot() {
        let mut pot = Pot::default();
        pot.collect_from_bets(&[(1, 100), (2, 100), (3, 100)]);
        let payouts = pot.distribute(&[1, 2]);
        let total_payout: u64 = payouts.iter().map(|(_, a)| a).sum();
        assert_eq!(total_payout, 300);
        assert_eq!(payouts.len(), 2);
        for &(_, amount) in &payouts {
            assert_eq!(amount, 150);
        }
    }

    #[test]
    fn test_distribute_three_way_split() {
        let mut pot = Pot::default();
        pot.collect_from_bets(&[(1, 100), (2, 100), (3, 100)]);
        let payouts = pot.distribute(&[1, 2, 3]);
        let total_payout: u64 = payouts.iter().map(|(_, a)| a).sum();
        assert_eq!(total_payout, 300);
    }

    #[test]
    fn test_distribute_with_side_pot() {
        let mut pot = Pot::default();
        pot.collect_from_bets(&[(1, 100), (2, 200), (3, 300)]);
        let payouts = pot.distribute(&[1]);
        let total: u64 = payouts.iter().map(|(_, a)| a).sum();
        assert_eq!(total, 600);
    }

    #[test]
    fn test_distribute_uneven_split_gives_remainder_to_first() {
        let mut pot = Pot::default();
        pot.collect_from_bets(&[(1, 100), (2, 100), (3, 100)]);
        let payouts = pot.distribute(&[1, 2]);
        let p1 = payouts.iter().find(|(id, _)| *id == 1).unwrap().1;
        let p2 = payouts.iter().find(|(id, _)| *id == 2).unwrap().1;
        assert_eq!(p1 + p2, 300);
        assert!(p1 >= p2);
    }
}
