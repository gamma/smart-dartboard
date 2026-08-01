use sdb_contracts::DartEvent;
use sdb_game_core::RegisteredGame;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct Fixture {
    #[serde(rename = "fixture_schema_version")]
    schema_version: u16,
    ruleset_version: u16,
    cases: Vec<FixtureCase>,
}

#[derive(Debug, Deserialize)]
struct FixtureCase {
    name: String,
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
}

#[test]
fn cricket_matches_shared_golden_fixture() {
    let fixture: Fixture =
        serde_json::from_str(include_str!("../../../fixtures/games/cricket_v1.json"))
            .expect("valid fixture");
    assert_eq!(fixture.schema_version, 1);
    assert_eq!(fixture.ruleset_version, 1);

    for case in fixture.cases {
        let players = case
            .players
            .into_iter()
            .map(|player| (player.id, player.name))
            .collect();
        let mut game = RegisteredGame::new("cricket", players, &case.options).expect("game");
        for step in case.steps {
            match step.command {
                FixtureCommand::Dart { event } => {
                    game.apply_throw(&event).expect("accepted dart");
                }
                FixtureCommand::Continue => {
                    game.continue_turn().expect("accepted continue");
                }
            }
            let state = game.state();
            let remaining_fields = state
                .overlay
                .pointer("/cricket/remaining")
                .and_then(Value::as_array)
                .expect("remaining targets")
                .iter()
                .filter_map(|item| item.get("field").and_then(Value::as_u64))
                .collect::<Vec<_>>();
            let actual = serde_json::json!({
                "scores": state.players.iter().map(|player| player.score).collect::<Vec<_>>(),
                "marks_20": state.players.iter().map(|player| player.marks.get("20").copied().unwrap_or_default()).collect::<Vec<_>>(),
                "current_player_index": state.current_player_index,
                "darts_in_turn": state.darts_in_turn,
                "turn_score": state.turn_score,
                "round_number": state.round_number,
                "status": state.status,
                "winner_id": state.winner_id,
                "result_type": state.result_type,
                "remaining_fields": remaining_fields,
            });
            assert_eq!(actual, step.expected, "fixture case {}", case.name);
        }
    }
}
