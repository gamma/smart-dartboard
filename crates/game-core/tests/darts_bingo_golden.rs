use sdb_contracts::DartEvent;
use sdb_game_core::RegisteredGame;
use serde::Deserialize;
use serde_json::Value;

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
fn darts_bingo_matches_shared_seeded_golden_fixture() {
    let fixture: Fixture =
        serde_json::from_str(include_str!("../../../fixtures/games/darts_bingo_v2.json"))
            .expect("fixture");
    assert_eq!(fixture.schema_version, 1);
    assert_eq!(fixture.ruleset_version, 2);
    let mut game = RegisteredGame::new_seeded(
        "darts_bingo",
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
        let tasks = state.mode_state["tasks"]
            .as_array()
            .expect("tasks")
            .iter()
            .map(|task| serde_json::json!([task["id"], task["label"]]))
            .collect::<Vec<_>>();
        let done = state
            .players
            .iter()
            .map(|player| {
                player
                    .marks
                    .iter()
                    .filter(|(_, value)| **value > 0)
                    .map(|(index, _)| index.parse::<u8>().expect("mark index"))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let actual = serde_json::json!({
            "scores":state.players.iter().map(|player|player.score).collect::<Vec<_>>(),
            "tasks":tasks,
            "done":done,
            "bingo_candidates":state.mode_state["bingo_candidates"],
            "last_effect":state.mode_state["last_effect"],
            "marked_count":state.mode_state["marked_count"],
            "effect_player_id":state.mode_state["effect_player_id"],
            "random_cursor":state.random_cursor,
            "current_player_index":state.current_player_index,
            "darts_in_turn":state.darts_in_turn,
            "turn_score":state.turn_score,
            "round_number":state.round_number,
            "status":state.status,
            "winner_id":state.winner_id,
            "winner_ids":state.winner_ids,
            "result_type":state.result_type,
        });
        assert_eq!(actual, step.expected);
    }
}
