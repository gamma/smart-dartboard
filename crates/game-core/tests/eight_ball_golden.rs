use sdb_contracts::DartEvent;
use sdb_game_core::RegisteredGame;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct Fixture {
    #[serde(rename = "fixture_schema_version")]
    schema_version: u16,
    ruleset_version: u16,
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
fn eight_ball_matches_shared_golden_fixture() {
    let fixture: Fixture =
        serde_json::from_str(include_str!("../../../fixtures/games/eight_ball_v1.json"))
            .expect("valid fixture");
    assert_eq!(fixture.schema_version, 1);
    assert_eq!(fixture.ruleset_version, 1);
    let players = fixture
        .players
        .into_iter()
        .map(|player| (player.id, player.name))
        .collect();
    let mut game = RegisteredGame::new("eight_ball", players, &fixture.options).expect("game");

    for step in fixture.steps {
        match step.command {
            FixtureCommand::Dart { event } => {
                game.apply_throw(&event).expect("accepted dart");
            }
            FixtureCommand::Continue => {
                game.continue_turn().expect("accepted continue");
            }
            FixtureCommand::Correct { action_id, event } => {
                game.correct_throw(action_id, event).expect("correction");
            }
        }
        let state = game.state();
        let player_id = &state.players[state.current_player_index].id;
        let remaining_balls = state.mode_state["balls"][player_id]
            .as_array()
            .expect("remaining balls");
        let actual = serde_json::json!({
            "scores": state.players.iter().map(|player| player.score).collect::<Vec<_>>(),
            "current_player_index": state.current_player_index,
            "darts_in_turn": state.darts_in_turn,
            "turn_score": state.turn_score,
            "round_number": state.round_number,
            "status": state.status,
            "winner_id": state.winner_id,
            "result_type": state.result_type,
            "remaining_balls": remaining_balls,
        });
        assert_eq!(actual, step.expected);
    }
}
