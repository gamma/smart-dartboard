use sdb_contracts::{DartEvent, PlayerRef};
use sdb_game_core::RegisteredGame;
use serde::Deserialize;
use serde_json::{Map, Value};

const SELECTED_CELLS: [&str; 5] = [
    "20:single_inner",
    "1:triple",
    "18:double",
    "25:single_bull",
    "25:double_bull",
];

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
fn king_of_board_matches_shared_golden_fixture() {
    let fixture: Fixture = serde_json::from_str(include_str!(
        "../../../fixtures/games/king_of_board_v2.json"
    ))
    .expect("fixture");
    assert_eq!(fixture.schema_version, 1);
    assert_eq!(fixture.ruleset_version, 2);
    let player_ids = fixture
        .players
        .iter()
        .map(|player| player.id.clone())
        .collect::<Vec<_>>();
    let mut game = RegisteredGame::new_seeded_with_players(
        "king_of_board",
        fixture.players,
        &fixture.options,
        fixture.random_seed,
    )
    .expect("game");
    assert_eq!(game.state().ruleset_version, fixture.ruleset_version);

    for step in fixture.steps {
        match step.command {
            Command::Dart { event } => game.apply_throw(&event).map(|_| ()),
            Command::Continue => game.continue_turn().map(|_| ()),
            Command::Correct { action_id, event } => {
                game.correct_throw(action_id, event).map(|_| ())
            }
        }
        .expect("command");
        let state = game.state();
        let owned = state.mode_state["owned"].as_object().expect("owned");
        let territory_counts = player_ids
            .iter()
            .map(|player_id| {
                let count = owned
                    .values()
                    .filter(|item| item["owner_id"].as_str() == Some(player_id))
                    .count();
                (player_id.clone(), Value::from(count))
            })
            .collect::<Map<_, _>>();
        let selected_owners = SELECTED_CELLS
            .iter()
            .filter_map(|cell| {
                let item = owned.get(*cell)?;
                Some((
                    (*cell).into(),
                    serde_json::json!([item["owner_id"], item["color"]]),
                ))
            })
            .collect::<Map<_, _>>();
        let actual = serde_json::json!({
            "scores": state.players.iter().map(|player| player.score).collect::<Vec<_>>(),
            "territory_counts": territory_counts,
            "selected_owners": selected_owners,
            "last_effect": state.mode_state["last_effect"],
            "capture_count": state.mode_state["capture_count"],
            "capture_cells": state.mode_state["capture_cells"],
            "previous_owner_ids": state.mode_state["previous_owner_ids"],
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
