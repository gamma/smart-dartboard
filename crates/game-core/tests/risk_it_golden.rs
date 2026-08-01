use sdb_contracts::DartEvent;
use sdb_game_core::RegisteredGame;
use serde::Deserialize;
use serde_json::{Map, Value};

#[derive(Deserialize)]
struct Fixture {
    #[serde(rename = "fixture_schema_version")]
    schema_version: u16,
    ruleset_version: u16,
    random_seed: u64,
    players: Vec<Player>,
    options: Value,
    steps: Vec<Step>,
}

#[derive(Deserialize)]
struct Player {
    id: String,
    name: String,
}

#[derive(Deserialize)]
struct Step {
    command: Command,
    expected: Value,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Command {
    Dart { event: DartEvent },
    Action { action: String, payload: Value },
    Continue,
    NextPlayer,
    Correct { action_id: u64, event: DartEvent },
}

#[test]
fn risk_it_matches_shared_golden_fixture() {
    let fixture: Fixture =
        serde_json::from_str(include_str!("../../../fixtures/games/risk_it_v3.json"))
            .expect("fixture");
    assert_eq!(fixture.schema_version, 1);
    assert_eq!(fixture.ruleset_version, 3);
    let player_ids = fixture
        .players
        .iter()
        .map(|player| player.id.clone())
        .collect::<Vec<_>>();
    let mut game = RegisteredGame::new_seeded(
        "risk_it",
        fixture
            .players
            .into_iter()
            .map(|player| (player.id, player.name))
            .collect(),
        &fixture.options,
        fixture.random_seed,
    )
    .expect("game");
    assert_eq!(game.state().ruleset_version, fixture.ruleset_version);

    for step in fixture.steps {
        match step.command {
            Command::Dart { event } => game.apply_throw(&event).map(|_| ()),
            Command::Action { action, payload } => {
                game.handle_action(&action, &payload).map(|_| ())
            }
            Command::Continue => game.continue_turn().map(|_| ()),
            Command::NextPlayer => game.next_player().map(|_| ()),
            Command::Correct { action_id, event } => {
                game.correct_throw(action_id, event).map(|_| ())
            }
        }
        .expect("command");
        let state = game.state();
        let pots = player_ids
            .iter()
            .map(|id| {
                (
                    id.clone(),
                    state.mode_state["pot"]
                        .get(id)
                        .cloned()
                        .unwrap_or(Value::from(0)),
                )
            })
            .collect::<Map<_, _>>();
        let actual = serde_json::json!({
            "scores": state.players.iter().map(|player| player.score).collect::<Vec<_>>(),
            "pots": pots,
            "hot_pot": state.mode_state["hot_pot"],
            "final_heist": state.mode_state["final_heist"],
            "banked_last": state.mode_state["banked_last"],
            "last_effect": state.mode_state["last_effect"],
            "effect_amount": state.mode_state["effect_amount"],
            "effect_target_player_id": state.mode_state["effect_target_player_id"],
            "current_player_index": state.current_player_index,
            "darts_in_turn": state.darts_in_turn,
            "turn_score": state.turn_score,
            "round_number": state.round_number,
            "status": state.status,
            "winner_id": state.winner_id,
            "result_type": state.result_type,
        });
        assert_eq!(actual, step.expected);
    }
}
