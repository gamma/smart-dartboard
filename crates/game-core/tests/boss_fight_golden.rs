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
fn boss_fight_matches_shared_seeded_golden_fixture() {
    let fixture: Fixture =
        serde_json::from_str(include_str!("../../../fixtures/games/boss_fight_v1.json"))
            .expect("fixture");
    assert_eq!(fixture.schema_version, 1);
    assert_eq!(fixture.ruleset_version, 1);
    let mut game = RegisteredGame::new_seeded(
        "boss_fight",
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
        let weak = state.mode_state["weak"]
            .as_array()
            .expect("weak")
            .iter()
            .map(|target| serde_json::json!([target["label"], target["field"], target["ring"]]))
            .collect::<Vec<_>>();
        let actual = serde_json::json!({
            "scores":state.players.iter().map(|player|player.score).collect::<Vec<_>>(),
            "boss_hp":state.mode_state["boss_hp"],
            "max_hp":state.mode_state["max_hp"],
            "weak":weak,
            "last_effect":state.mode_state["last_effect"],
            "effect_damage":state.mode_state["effect_damage"],
            "effect_weak":state.mode_state["effect_weak"],
            "effect_player_id":state.mode_state["effect_player_id"],
            "random_cursor":state.random_cursor,
            "current_player_index":state.current_player_index,
            "darts_in_turn":state.darts_in_turn,
            "turn_score":state.turn_score,
            "round_number":state.round_number,
            "status":state.status,
            "winner_ids":state.winner_ids,
            "result_type":state.result_type,
        });
        assert_eq!(actual, step.expected);
    }
}
