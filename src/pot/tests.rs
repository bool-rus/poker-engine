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
