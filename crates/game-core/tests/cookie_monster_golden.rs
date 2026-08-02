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
fn cookie_monster_matches_shared_seeded_golden_fixture() {
    let fixture: Fixture = serde_json::from_str(include_str!(
        "../../../fixtures/games/cookie_monster_v2.json"
    ))
    .expect("fixture");
    assert_eq!(fixture.schema_version, 1);
    assert_eq!(fixture.ruleset_version, 2);
    let mut game = RegisteredGame::new_seeded_with_players(
        "cookie_monster",
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
        let mut actual = snapshot(game.state());
        let mut expected = step.expected;
        normalize_layout(&mut actual);
        normalize_layout(&mut expected);
        assert_eq!(actual, expected);
    }
}

fn normalize_layout(snapshot: &mut Value) {
    snapshot["layout"]
        .as_array_mut()
        .expect("layout")
        .sort_by(|left, right| left[0].as_str().cmp(&right[0].as_str()));
}

fn snapshot(state: &sdb_game_core::RegisteredGameState) -> Value {
    let values = |key: &str| {
        state
            .players
            .iter()
            .map(|player| state.mode_state[key][&player.id].clone())
            .collect::<Vec<_>>()
    };
    let wave = state.mode_state["wave"][&state.players[0].id]
        .as_u64()
        .expect("wave");
    let layout = state.mode_state["layouts"][wave.to_string()]
        .as_object()
        .expect("layout")
        .iter()
        .map(|(id, item)| serde_json::json!([id, item["kind"]]))
        .collect::<Vec<_>>();
    serde_json::json!({
        "scores": state.players.iter().map(|player| player.score).collect::<Vec<_>>(),
        "streak": values("streak"), "sugar": values("sugar"), "wave": values("wave"),
        "collected": values("collected"), "layout": layout,
        "last_effect": state.mode_state["last_effect"], "effect_points": state.mode_state["effect_points"],
        "cookie_wave": state.mode_state["cookie_wave"], "random_cursor": state.random_cursor,
        "current_player_index": state.current_player_index, "darts_in_turn": state.darts_in_turn,
        "turn_score": state.turn_score, "round_number": state.round_number, "status": state.status,
    })
}
