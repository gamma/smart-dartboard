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
    Continue,
    Correct { action_id: u64, event: DartEvent },
}

#[test]
fn treasure_hunt_matches_shared_seeded_golden_fixture() {
    let fixture: Fixture = serde_json::from_str(include_str!(
        "../../../fixtures/games/treasure_hunt_v1.json"
    ))
    .expect("fixture");
    assert_eq!(fixture.schema_version, 1);
    assert_eq!(fixture.ruleset_version, 1);
    let mut game = RegisteredGame::new_seeded(
        "treasure_hunt",
        fixture
            .players
            .into_iter()
            .map(|player| (player.id, player.name))
            .collect(),
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
        let state = game.state();
        let revealed = state.mode_state["revealed"]
            .as_object()
            .expect("revealed")
            .iter()
            .map(|(key, item)| {
                (
                    key.clone(),
                    serde_json::json!({
                        "reward": item["reward"],
                        "revealed_by": item["revealed_by"],
                    }),
                )
            })
            .collect::<Map<_, _>>();
        let actual = serde_json::json!({
            "scores": state.players.iter().map(|player| player.score).collect::<Vec<_>>(),
            "revealed": revealed,
            "current_player_index": state.current_player_index,
            "darts_in_turn": state.darts_in_turn,
            "turn_score": state.turn_score,
            "round_number": state.round_number,
            "status": state.status,
            "winner_id": state.winner_id,
            "result_type": state.result_type,
            "random_cursor": state.random_cursor,
        });
        assert_eq!(actual, step.expected);
    }
}
