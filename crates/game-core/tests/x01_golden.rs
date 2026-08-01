use sdb_contracts::DartEvent;
use sdb_game_core::{OutRule, X01Game};
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
    options: FixtureOptions,
    setup_current_score: u32,
    steps: Vec<FixtureStep>,
}

#[derive(Debug, Deserialize)]
struct FixturePlayer {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct FixtureOptions {
    start_score: u32,
    out_rule: OutRule,
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
    Delete { action_id: u64 },
    Undo,
}

#[test]
fn x01_matches_shared_golden_fixture() {
    let fixture: Fixture =
        serde_json::from_str(include_str!("../../../fixtures/games/x01_v1.json"))
            .expect("valid fixture");
    assert_eq!(fixture.schema_version, 1);
    assert_eq!(fixture.ruleset_version, 1);

    for case in fixture.cases {
        let players = case
            .players
            .into_iter()
            .map(|player| (player.id, player.name))
            .collect();
        let start_score = if case.setup_current_score == case.options.start_score {
            case.options.start_score
        } else {
            case.setup_current_score
        };
        let mut game = X01Game::new(players, start_score, case.options.out_rule).expect("game");

        for step in case.steps {
            match step.command {
                FixtureCommand::Dart { event } => {
                    game.apply_throw(event).expect("accepted dart");
                }
                FixtureCommand::Continue => {
                    game.continue_turn().expect("accepted continue");
                }
                FixtureCommand::Correct { action_id, event } => {
                    game.correct_throw(action_id, event).expect("correction");
                }
                FixtureCommand::Delete { action_id } => {
                    game.delete_throw(action_id).expect("deletion");
                }
                FixtureCommand::Undo => {
                    game.undo().expect("undo");
                }
            }

            let state = game.state();
            let dart_actions = game.dart_actions();
            let actual = serde_json::json!({
                "scores": state.players.iter().map(|player| player.score).collect::<Vec<_>>(),
                "current_player_index": state.current_player_index,
                "darts_in_turn": state.darts_in_turn,
                "turn_score": state.turn_score,
                "round_number": state.round_number,
                "status": state.status,
                "winner_id": state.winner_id,
                "result_type": state.result_type,
                "bust": state.last_bust,
                "labels": dart_actions.iter().map(|(_, event)| event.label()).collect::<Vec<_>>(),
                "seqs": dart_actions.iter().map(|(_, event)| event.seq()).collect::<Vec<_>>(),
            });
            assert_eq!(actual, step.expected, "fixture case {}", case.name);
        }
    }
}
