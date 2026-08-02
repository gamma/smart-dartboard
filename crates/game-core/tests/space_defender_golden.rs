use sdb_contracts::{DartEvent, PlayerRef};
use sdb_game_core::RegisteredGame;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
struct Fixture {
    #[serde(rename = "fixture_schema_version")]
    schema_version: u16,
    ruleset_version: u16,
    random_seed: u64,
    players: Vec<PlayerRef>,
    options: Value,
    steps: Vec<Step>,
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
    Continue,
    Correct { action_id: u64, event: DartEvent },
}

#[test]
fn space_defender_matches_shared_seeded_golden_fixture() {
    let fixture: Fixture = serde_json::from_str(include_str!(
        "../../../fixtures/games/space_defender_v2.json"
    ))
    .expect("fixture");
    assert_eq!(fixture.schema_version, 1);
    assert_eq!(fixture.ruleset_version, 2);
    let mut game = RegisteredGame::new_seeded_with_players(
        "space_defender",
        fixture.players,
        &fixture.options,
        fixture.random_seed,
    )
    .expect("game");

    for step in fixture.steps {
        match step.command {
            Command::Dart { event } => game.apply_throw(&event).map(|_| ()),
            Command::Continue => game.continue_turn().map(|_| ()),
            Command::Correct { action_id, event } => {
                game.correct_throw(action_id, event).map(|_| ())
            }
        }
        .expect("command");
        assert_eq!(snapshot(game.state()), step.expected);
    }
}

fn snapshot(state: &sdb_game_core::RegisteredGameState) -> Value {
    let ships = state.mode_state["ships"]
        .as_array()
        .expect("ships")
        .iter()
        .map(|ship| {
            serde_json::json!({
                "id": ship["id"], "type": ship["type"], "target": ship["target"]["label"],
                "hp": ship["hp"], "max_hp": ship["max_hp"], "points": ship["points"],
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "scores": state.players.iter().map(|player| player.score).collect::<Vec<_>>(),
        "ships": ships, "wave": state.mode_state["wave"], "cleanup": state.mode_state["cleanup"],
        "next_ship_id": state.mode_state["next_ship_id"], "last_effect": state.mode_state["last_effect"],
        "effect_points": state.mode_state["effect_points"], "effect_damage": state.mode_state["effect_damage"],
        "destroyed": state.mode_state["destroyed"], "random_cursor": state.random_cursor,
        "current_player_index": state.current_player_index, "darts_in_turn": state.darts_in_turn,
        "turn_score": state.turn_score, "round_number": state.round_number, "status": state.status,
        "winner_ids": state.winner_ids, "result_type": state.result_type,
    })
}
