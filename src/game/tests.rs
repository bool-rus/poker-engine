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
    // Adding a player during an active game is allowed — they sit without cards
    assert!(game.add_player(3).is_ok());
    let p3 = game.all_players().iter().find(|p| p.id == 3).unwrap();
    assert_eq!(p3.status, PlayerStatus::SittingOut);
    assert!(p3.wants_in);
}

#[test]
fn test_add_player_during_game_no_cards_current_hand() {
    let mut game = setup_two_player_game();
    let cmds = game.start_hand().unwrap();
    // Only 2 DealHoleCards — new player not yet added
    assert_eq!(cmds.len(), 2);

    // Add player 3 mid-game
    game.add_player(3).unwrap();

    // Hand continues normally — no extra DealHoleCards for player 3
    game.handle_event(GameEvent::HoleCardsDealt { player_id: 1 }).unwrap();
    game.handle_event(GameEvent::HoleCardsDealt { player_id: 2 }).unwrap();

    // Player 3 is not in eligible players
    let active = game.active_player().unwrap();
    assert!(active == 1 || active == 2);
}

#[test]
fn test_add_player_during_game_gets_cards_next_hand() {
    let mut game = setup_two_player_game();

    // Hand 1
    game.start_hand().unwrap();
    game.handle_event(GameEvent::HoleCardsDealt { player_id: 1 }).unwrap();
    game.handle_event(GameEvent::HoleCardsDealt { player_id: 2 }).unwrap();
    let a = game.active_player().unwrap();
    game.player_action(a, PlayerAction::Fold).unwrap();

    // Add player 3 after hand 1
    game.add_player(3).unwrap();

    // Hand 2 — player 3 gets cards
    let cmds = game.start_hand().unwrap();
    let ids: Vec<PlayerId> = cmds.iter().map(|c| match c {
        GameCommand::DealHoleCards { player_id } => *player_id,
        _ => panic!("expected DealHoleCards"),
    }).collect();
    assert!(ids.contains(&3), "player 3 should get cards in next hand");
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

// === Blind skipping for busted players ===

fn bust_player(game: &mut Game, loser_id: PlayerId) {
    game.start_hand().unwrap();
    let ids: Vec<PlayerId> = game.all_players().iter()
        .filter(|p| p.status == PlayerStatus::Active && p.chips > 0)
        .map(|p| p.id)
        .collect();
    for id in &ids {
        game.handle_event(GameEvent::HoleCardsDealt { player_id: *id }).unwrap();
    }
    // Everyone goes all-in
    while let Some(id) = game.active_player() {
        game.player_action(id, PlayerAction::AllIn).unwrap();
    }
    // Process auto-advance through community cards to showdown
    while game.phase() != GamePhase::Showdown && game.phase() != GamePhase::GameOver {
        let _ = game.handle_event(GameEvent::CommunityCardsRevealed);
    }
    if game.phase() == GamePhase::Showdown {
        let showdown_ids: Vec<PlayerId> = game.all_players().iter()
            .filter(|p| p.is_active_in_hand())
            .map(|p| p.id)
            .collect();
        for id in &showdown_ids {
            let score = if *id == loser_id { 0 } else { 200 };
            game.handle_event(GameEvent::PlayerCardsRevealed { player_id: *id, score }).unwrap();
        }
    }
}

#[test]
fn test_skip_busted_player_for_blinds() {
    let mut game = Game::new(GameConfig {
        small_blind: 50,
        big_blind: 100,
        starting_chips: 100,
        max_players: 9,
        min_players: 2,
        allow_rebuy: false,
        rebuy_amount: None,
    });
    game.add_player(1).unwrap();
    game.add_player(2).unwrap();
    game.add_player(3).unwrap();

    bust_player(&mut game, 3);

    let p3 = game.all_players().iter().find(|p| p.id == 3).unwrap();
    assert_eq!(p3.chips, 0);

    game.start_hand().unwrap();

    let p3 = game.all_players().iter().find(|p| p.id == 3).unwrap();
    assert_eq!(p3.bet, 0, "busted player should not post blind");
    assert!(!p3.is_dealer, "busted player should not be dealer");

    let active = game.active_player().unwrap();
    assert_ne!(active, 3, "busted player should not be first to act");
}

#[test]
fn test_all_in_posting_when_chips_less_than_blind() {
    let mut game = Game::new(GameConfig {
        small_blind: 50,
        big_blind: 100,
        starting_chips: 100,
        max_players: 9,
        min_players: 2,
        allow_rebuy: true,
        rebuy_amount: Some(100),
    });
    game.add_player(1).unwrap();
    game.add_player(2).unwrap();

    bust_player(&mut game, 2);

    assert_eq!(game.all_players().iter().find(|p| p.id == 2).unwrap().chips, 0);

    game.rebuy(2, 30).unwrap();
    assert_eq!(game.all_players().iter().find(|p| p.id == 2).unwrap().chips, 30);

    game.start_hand().unwrap();

    let p2 = game.all_players().iter().find(|p| p.id == 2).unwrap();
    assert_eq!(p2.bet, 30, "player with less than blind posts all-in");
    assert!(p2.all_in);
}

#[test]
fn test_two_eligible_three_total_blinds() {
    let mut game = Game::new(GameConfig {
        small_blind: 50,
        big_blind: 100,
        starting_chips: 100,
        max_players: 9,
        min_players: 2,
        allow_rebuy: false,
        rebuy_amount: None,
    });
    game.add_player(1).unwrap();
    game.add_player(2).unwrap();
    game.add_player(3).unwrap();

    bust_player(&mut game, 3);

    assert_eq!(game.all_players().iter().find(|p| p.id == 3).unwrap().chips, 0);

    game.start_hand().unwrap();

    let p3 = game.all_players().iter().find(|p| p.id == 3).unwrap();
    assert_eq!(p3.bet, 0);

    let total_bets: u64 = game.all_players().iter()
        .filter(|p| p.status == PlayerStatus::Active && p.chips > 0)
        .map(|p| p.bet)
        .sum();
    assert_eq!(total_bets, game.config().small_blind + game.config().big_blind);
}

#[test]
fn test_heads_up_when_third_player_busted() {
    let mut game = Game::new(GameConfig {
        small_blind: 50,
        big_blind: 100,
        starting_chips: 100,
        max_players: 9,
        min_players: 2,
        allow_rebuy: false,
        rebuy_amount: None,
    });
    game.add_player(1).unwrap();
    game.add_player(2).unwrap();
    game.add_player(3).unwrap();

    bust_player(&mut game, 3);

    assert_eq!(game.all_players().iter().find(|p| p.id == 3).unwrap().chips, 0);

    game.start_hand().unwrap();

    let active = game.active_player().unwrap();
    let sb = game.all_players().iter()
        .find(|p| p.bet == game.config().small_blind)
        .unwrap();
    assert_eq!(active, sb.id, "SB should act first in heads-up");
}

// === Multi-player all-in showdown with different stacks ===

#[test]
fn test_three_player_allin_different_stacks() {
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

    game.start_hand().unwrap();
    game.handle_event(GameEvent::HoleCardsDealt { player_id: 1 }).unwrap();
    game.handle_event(GameEvent::HoleCardsDealt { player_id: 2 }).unwrap();
    game.handle_event(GameEvent::HoleCardsDealt { player_id: 3 }).unwrap();

    // All three go all-in
    let a = game.active_player().unwrap();
    game.player_action(a, PlayerAction::AllIn).unwrap();
    let a = game.active_player().unwrap();
    game.player_action(a, PlayerAction::AllIn).unwrap();
    let a = game.active_player().unwrap();
    game.player_action(a, PlayerAction::AllIn).unwrap();

    // Auto-advance to showdown
    while game.phase() != GamePhase::Showdown && game.phase() != GamePhase::GameOver {
        let _ = game.handle_event(GameEvent::CommunityCardsRevealed);
    }
    assert_eq!(game.phase(), GamePhase::Showdown);

    let pot_before = game.pot_total();

    // Showdown — player 3 wins (highest score)
    game.handle_event(GameEvent::PlayerCardsRevealed { player_id: 1, score: 100 }).unwrap();
    game.handle_event(GameEvent::PlayerCardsRevealed { player_id: 2, score: 200 }).unwrap();
    game.handle_event(GameEvent::PlayerCardsRevealed { player_id: 3, score: 300 }).unwrap();

    assert_eq!(game.phase(), GamePhase::GameOver);

    let p3 = game.all_players().iter().find(|p| p.id == 3).unwrap();
    assert_eq!(p3.chips, pot_before, "winner takes the entire pot");

    let p1 = game.all_players().iter().find(|p| p.id == 1).unwrap();
    let p2 = game.all_players().iter().find(|p| p.id == 2).unwrap();
    assert_eq!(p1.chips, 0);
    assert_eq!(p2.chips, 0);
}

#[test]
fn test_three_player_allin_pot_total() {
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

    while game.phase() != GamePhase::Showdown && game.phase() != GamePhase::GameOver {
        let _ = game.handle_event(GameEvent::CommunityCardsRevealed);
    }

    // Pot should be 3000 (3 × 1000)
    assert_eq!(game.pot_total(), 3000);

    // Player 2 wins
    game.handle_event(GameEvent::PlayerCardsRevealed { player_id: 1, score: 100 }).unwrap();
    game.handle_event(GameEvent::PlayerCardsRevealed { player_id: 2, score: 300 }).unwrap();
    game.handle_event(GameEvent::PlayerCardsRevealed { player_id: 3, score: 200 }).unwrap();

    let p2 = game.all_players().iter().find(|p| p.id == 2).unwrap();
    assert_eq!(p2.chips, 3000, "winner takes entire pot");

    let p1 = game.all_players().iter().find(|p| p.id == 1).unwrap();
    let p3 = game.all_players().iter().find(|p| p.id == 3).unwrap();
    assert_eq!(p1.chips, 0);
    assert_eq!(p3.chips, 0);
}
