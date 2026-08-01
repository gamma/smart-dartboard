use sdb_contracts::PlayerRef;
use sdb_session_core::SessionCore;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
struct Fixture {
    #[serde(rename = "fixture_schema_version")]
    schema_version: u16,
    ruleset_version: u16,
    cases: Vec<FixtureCase>,
}

#[derive(Deserialize)]
struct FixtureCase {
    name: String,
    players: Vec<PlayerRef>,
    steps: Vec<FixtureStep>,
}

#[derive(Deserialize)]
struct FixtureStep {
    command: FixtureCommand,
    expected: Value,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum FixtureCommand {
    StartSession,
    Prepare {
        game_type: String,
        options: Value,
    },
    StartGame {
        game_id: String,
    },
    Playing,
    Complete {
        winner_ids: Vec<String>,
        #[serde(rename = "result_type")]
        _result_type: String,
    },
    NextGame,
    Abort,
    EndSession,
    Rematch {
        game_id: String,
    },
}

#[test]
fn session_flow_matches_shared_golden_fixture() {
    let fixture: Fixture =
        serde_json::from_str(include_str!("../../../fixtures/sessions/session_v1.json"))
            .expect("valid fixture");
    assert_eq!(fixture.schema_version, 1);
    assert_eq!(fixture.ruleset_version, 1);

    for case in fixture.cases {
        let expected_player_ids: Vec<String> = case
            .players
            .iter()
            .map(|player| player.id.clone())
            .collect();
        let mut session = SessionCore::default();
        for step in case.steps {
            match step.command {
                FixtureCommand::StartSession => {
                    session
                        .start_session("session-1", case.players.clone())
                        .expect("start session");
                }
                FixtureCommand::Prepare { game_type, options } => {
                    session
                        .prepare_game(game_type, options)
                        .expect("prepare game");
                }
                FixtureCommand::StartGame { game_id } => {
                    session.start_game(game_id).expect("start game");
                }
                FixtureCommand::Playing => {
                    session.mark_playing().expect("mark playing");
                }
                FixtureCommand::Complete { winner_ids, .. } => {
                    session.complete_game(&winner_ids).expect("complete game");
                }
                FixtureCommand::NextGame => {
                    session.next_game().expect("next game");
                }
                FixtureCommand::Abort => {
                    session.abort_game().expect("abort game");
                }
                FixtureCommand::EndSession => {
                    session.end_session().expect("end session");
                }
                FixtureCommand::Rematch { game_id } => {
                    session.start_rematch(game_id).expect("start rematch");
                }
            }

            let state = session.state();
            let actual = serde_json::json!({
                "screen": state.screen,
                "session_status": state.session_status,
                "selected_mode": state.prepared_game.as_ref().map(|game| &game.game_type),
                "starter_id": state.selected_starter_id,
                "starter_selection": state.starter_selection,
                "game_active": state.game_id.is_some(),
                "lineup": state.game_player_ids,
                "standings": expected_player_ids.iter().map(|player_id| {
                    let standing = state.standings.iter()
                        .find(|standing| &standing.player_id == player_id)
                        .expect("standing");
                    serde_json::json!({
                        "id": player_id,
                        "games": standing.games,
                        "wins": standing.wins,
                        "session_points": standing.session_points,
                    })
                }).collect::<Vec<_>>(),
            });
            assert_eq!(actual, step.expected, "fixture case {}", case.name);
        }
    }
}
