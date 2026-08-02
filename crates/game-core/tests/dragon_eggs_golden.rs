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
fn dragon_eggs_matches_shared_seeded_golden_fixture() {
    let fixture: Fixture =
        serde_json::from_str(include_str!("../../../fixtures/games/dragon_eggs_v2.json"))
            .expect("fixture");
    assert_eq!(fixture.schema_version, 1);
    assert_eq!(fixture.ruleset_version, 2);
    let mut game = RegisteredGame::new_seeded_with_players(
        "dragon_eggs",
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
        assert_eq!(snapshot(game.state()), step.expected);
    }
}

fn snapshot(state: &sdb_game_core::RegisteredGameState) -> Value {
    let values = |key: &str| {
        state
            .players
            .iter()
            .map(|player| state.mode_state[key][&player.id].clone())
            .collect::<Vec<_>>()
    };
    let labels = |key: &str| {
        state.mode_state[key]
            .as_array()
            .expect("targets")
            .iter()
            .map(|target| target["label"].clone())
            .collect::<Vec<_>>()
    };
    serde_json::json!({
        "scores": state.players.iter().map(|player| player.score).collect::<Vec<_>>(),
        "heat": values("heat"), "positive": values("turn_positive"), "collected": values("collected"),
        "eggs": labels("eggs"), "scales": labels("scales"),
        "layout_round": state.mode_state["layout_round"], "last_effect": state.mode_state["last_effect"],
        "effect_points": state.mode_state["effect_points"], "dragon_heat": state.mode_state["dragon_heat"],
        "dragon_fire_penalty": state.mode_state["dragon_fire_penalty"], "random_cursor": state.random_cursor,
        "current_player_index": state.current_player_index, "darts_in_turn": state.darts_in_turn,
        "turn_score": state.turn_score, "round_number": state.round_number, "status": state.status,
    })
}
