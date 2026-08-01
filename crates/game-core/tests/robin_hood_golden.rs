use sdb_contracts::DartEvent;
use sdb_game_core::{GameStatus, RegisteredGame};
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
fn robin_hood_matches_shared_seeded_golden_fixture() {
    let fixture: Fixture =
        serde_json::from_str(include_str!("../../../fixtures/games/robin_hood_v2.json"))
            .expect("valid fixture");
    assert_eq!(fixture.schema_version, 1);
    assert_eq!(fixture.ruleset_version, 2);
    let players = fixture
        .players
        .into_iter()
        .map(|player| (player.id, player.name))
        .collect();
    let mut game =
        RegisteredGame::new_seeded("robin_hood", players, &fixture.options, fixture.random_seed)
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
        let labels = |name: &str| {
            state.mode_state[name]
                .as_array()
                .expect("target list")
                .iter()
                .map(|target| target["label"].as_str().expect("label"))
                .collect::<Vec<_>>()
        };
        let shown_targets = if state.status == GameStatus::Hold {
            labels("sheriff_targets")
        } else {
            labels("remaining_targets")
        };
        let actual = serde_json::json!({
            "scores": state.players.iter().map(|player| player.score).collect::<Vec<_>>(),
            "splits": state.players.iter().map(|player| state.mode_state["splits"][&player.id].as_u64().unwrap_or_default()).collect::<Vec<_>>(),
            "sheriff_targets": labels("sheriff_targets"),
            "remaining_targets": labels("remaining_targets"),
            "current_arrows": labels("current_arrows"),
            "shown_targets": shown_targets,
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
