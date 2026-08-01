use sdb_contracts::DartEvent;
use sdb_game_core::RegisteredGame;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct Fixture {
    #[serde(rename = "fixture_schema_version")]
    schema_version: u16,
    ruleset_version: u16,
    random_seed: u64,
    players: Vec<FixturePlayer>,
    options: Value,
    steps: Vec<FixtureStep>,
}

#[derive(Debug, Deserialize)]
struct FixturePlayer {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct FixtureStep {
    command: FixtureCommand,
    expected: Value,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum FixtureCommand {
    Dart { event: DartEvent },
    Continue,
    Correct { action_id: u64, event: DartEvent },
}

#[test]
fn ghost_chase_matches_shared_seeded_golden_fixture() {
    let fixture: Fixture =
        serde_json::from_str(include_str!("../../../fixtures/games/ghost_chase_v2.json"))
            .expect("valid fixture");
    assert_eq!(fixture.schema_version, 1);
    assert_eq!(fixture.ruleset_version, 2);
    let players = fixture
        .players
        .into_iter()
        .map(|player| (player.id, player.name))
        .collect();
    let mut game = RegisteredGame::new_seeded(
        "ghost_chase",
        players,
        &fixture.options,
        fixture.random_seed,
    )
    .expect("game");

    for step in fixture.steps {
        match step.command {
            FixtureCommand::Dart { event } => {
                game.apply_throw(&event).expect("accepted dart");
            }
            FixtureCommand::Continue => {
                game.continue_turn().expect("accepted continue");
            }
            FixtureCommand::Correct { action_id, event } => {
                game.correct_throw(action_id, event)
                    .expect("accepted correction");
            }
        }
        let state = game.state();
        let counters = |name: &str| {
            state
                .players
                .iter()
                .map(|player| {
                    state.mode_state[name][&player.id]
                        .as_u64()
                        .expect("counter")
                })
                .collect::<Vec<_>>()
        };
        let path = state.mode_state["path"]
            .as_array()
            .expect("path")
            .iter()
            .map(|target| target["label"].as_str().expect("label"))
            .collect::<Vec<_>>();
        let actual = serde_json::json!({
            "scores": state.players.iter().map(|player| player.score).collect::<Vec<_>>(),
            "combos": counters("combo"),
            "escapes": counters("escape"),
            "path_indices": counters("path_index"),
            "path": path,
            "active_target": state.overlay["targets"][0]["id"],
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
