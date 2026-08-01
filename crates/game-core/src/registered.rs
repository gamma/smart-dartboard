use crate::{GameError, GameStatus};
use sdb_contracts::{DartEvent, Ring};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

const CRICKET_TARGETS: [u8; 7] = [20, 19, 18, 17, 16, 15, 25];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameOptionValue {
    Integer(i64),
    Boolean(bool),
    Text(&'static str),
}

impl Serialize for GameOptionValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Integer(value) => serializer.serialize_i64(*value),
            Self::Boolean(value) => serializer.serialize_bool(*value),
            Self::Text(value) => serializer.serialize_str(value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct GameOptionChoice {
    pub value: GameOptionValue,
    pub label: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_en: Option<&'static str>,
}

impl GameOptionValue {
    fn as_json(self) -> Value {
        match self {
            Self::Integer(value) => Value::from(value),
            Self::Boolean(value) => Value::from(value),
            Self::Text(value) => Value::from(value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct GameOption {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: &'static str,
    pub default: GameOptionValue,
    pub choices: &'static [GameOptionChoice],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct GameInstruction {
    pub title: &'static str,
    pub body: &'static str,
    pub icon: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct GameMetadata {
    pub slug: &'static str,
    pub ruleset_version: u16,
    pub title: &'static str,
    pub tagline: &'static str,
    pub description: &'static str,
    pub accent: &'static str,
    pub accent_secondary: &'static str,
    pub visual: &'static str,
    pub icon: &'static str,
    pub artwork: &'static str,
    pub sound_theme: &'static str,
    pub min_players: usize,
    pub max_players: usize,
    pub options: &'static [GameOption],
    pub instructions: &'static [GameInstruction],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisteredPlayer {
    pub id: String,
    pub name: String,
    pub score: u32,
    #[serde(default)]
    pub marks: BTreeMap<String, u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisteredGameState {
    pub game_type: String,
    pub ruleset_version: u16,
    pub players: Vec<RegisteredPlayer>,
    pub current_player_index: usize,
    pub darts_in_turn: u8,
    pub turn_score: u32,
    pub round_number: u16,
    pub status: GameStatus,
    pub winner_id: Option<String>,
    pub winner_ids: Vec<String>,
    pub result_type: String,
    pub message: String,
    pub options: Value,
    pub overlay: Value,
    pub last_event: Option<DartEvent>,
    #[serde(default)]
    pub mode_state: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisteredGame {
    state: RegisteredGameState,
    history: Vec<RegisteredGameState>,
}

impl RegisteredGame {
    /// Creates a game from the build-time registry and resolves its options.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown mode, invalid players or invalid options.
    pub fn new(
        game_type: &str,
        players: Vec<(String, String)>,
        options: &Value,
    ) -> Result<Self, GameError> {
        if players.is_empty() {
            return Err(GameError::NoPlayers);
        }
        let mode = mode(game_type)?;
        let metadata = mode.metadata();
        if !(metadata.min_players..=metadata.max_players).contains(&players.len()) {
            return Err(GameError::InvalidOptions(format!(
                "{} supports {} to {} players",
                metadata.slug, metadata.min_players, metadata.max_players
            )));
        }
        let options = resolve_options(metadata, options)?;
        let mut state = RegisteredGameState {
            game_type: metadata.slug.into(),
            ruleset_version: metadata.ruleset_version,
            players: players
                .into_iter()
                .map(|(id, name)| RegisteredPlayer {
                    id,
                    name,
                    score: 0,
                    marks: BTreeMap::new(),
                })
                .collect(),
            current_player_index: 0,
            darts_in_turn: 0,
            turn_score: 0,
            round_number: 1,
            status: GameStatus::Running,
            winner_id: None,
            winner_ids: Vec::new(),
            result_type: String::new(),
            message: "Game started".into(),
            options,
            overlay: Value::Null,
            last_event: None,
            mode_state: Value::Object(Map::new()),
        };
        mode.initialize(&mut state)?;
        state.overlay = mode.overlay(&state);
        Ok(Self {
            state,
            history: Vec::new(),
        })
    }

    #[must_use]
    pub const fn state(&self) -> &RegisteredGameState {
        &self.state
    }

    /// Applies one canonical dart event to the registered ruleset.
    ///
    /// # Errors
    ///
    /// Returns an error when the game is not running or its ruleset is unavailable.
    pub fn apply_throw(&mut self, event: &DartEvent) -> Result<&RegisteredGameState, GameError> {
        if self.state.status != GameStatus::Running {
            return Err(GameError::NotRunning);
        }
        let mode = self.resolve_mode()?;
        let mut next = self.state.clone();
        let turn_value = mode.apply_throw(&mut next, event)?;
        next.darts_in_turn = next.darts_in_turn.saturating_add(1);
        next.turn_score = next.turn_score.saturating_add(turn_value);
        next.last_event = Some(event.clone());
        if next.status == GameStatus::Running && next.darts_in_turn >= 3 {
            next.status = GameStatus::Hold;
            next.message = "Turn complete. Press continue.".into();
        }
        next.overlay = mode.overlay(&next);
        self.history.push(self.state.clone());
        self.state = next;
        Ok(&self.state)
    }

    /// Advances a held game to the next player's turn.
    ///
    /// # Errors
    ///
    /// Returns an error unless the game is holding or its ruleset is unavailable.
    pub fn continue_turn(&mut self) -> Result<&RegisteredGameState, GameError> {
        if self.state.status != GameStatus::Hold {
            return Err(GameError::NotHolding);
        }
        let mode = self.resolve_mode()?;
        let mut next = self.state.clone();
        let last_player = next.current_player_index + 1 == next.players.len();
        next.current_player_index = (next.current_player_index + 1) % next.players.len();
        if last_player {
            next.round_number = next.round_number.saturating_add(1);
        }
        next.darts_in_turn = 0;
        next.turn_score = 0;
        next.status = GameStatus::Running;
        next.message = "Next player".into();
        next.last_event = None;
        mode.on_turn_started(&mut next)?;
        next.overlay = mode.overlay(&next);
        self.history.push(self.state.clone());
        self.state = next;
        Ok(&self.state)
    }

    /// Applies a mode-defined non-dart action.
    ///
    /// # Errors
    ///
    /// Returns an error for unsupported actions, invalid payloads or missing rulesets.
    pub fn handle_action(
        &mut self,
        action: &str,
        payload: &Value,
    ) -> Result<&RegisteredGameState, GameError> {
        let mode = self.resolve_mode()?;
        let mut next = self.state.clone();
        mode.handle_action(&mut next, action, payload)?;
        next.overlay = mode.overlay(&next);
        self.history.push(self.state.clone());
        self.state = next;
        Ok(&self.state)
    }

    /// Restores the state before the latest accepted throw, turn or mode action.
    ///
    /// # Errors
    ///
    /// Returns [`GameError::NothingToUndo`] when no prior state exists.
    pub fn undo(&mut self) -> Result<&RegisteredGameState, GameError> {
        self.state = self.history.pop().ok_or(GameError::NothingToUndo)?;
        Ok(&self.state)
    }

    fn resolve_mode(&self) -> Result<&'static dyn GameMode, GameError> {
        let mode = mode(&self.state.game_type)?;
        if mode.metadata().ruleset_version != self.state.ruleset_version {
            return Err(GameError::RulesetUnavailable(format!(
                "{} version {}",
                self.state.game_type, self.state.ruleset_version
            )));
        }
        Ok(mode)
    }
}

trait GameMode: Sync {
    fn metadata(&self) -> &'static GameMetadata;
    fn initialize(&self, state: &mut RegisteredGameState) -> Result<(), GameError>;
    fn apply_throw(
        &self,
        state: &mut RegisteredGameState,
        event: &DartEvent,
    ) -> Result<u32, GameError>;
    fn overlay(&self, state: &RegisteredGameState) -> Value;

    fn on_turn_started(&self, _state: &mut RegisteredGameState) -> Result<(), GameError> {
        Ok(())
    }

    fn handle_action(
        &self,
        _state: &mut RegisteredGameState,
        action: &str,
        _payload: &Value,
    ) -> Result<(), GameError> {
        Err(GameError::UnsupportedAction(action.into()))
    }
}

struct CricketMode;

static CRICKET_INSTRUCTIONS: [GameInstruction; 4] = [
    GameInstruction {
        title: "Ziele treffen",
        body: "Nur 15, 16, 17, 18, 19, 20 und Bull zählen.",
        icon: "targets",
    },
    GameInstruction {
        title: "Dreimal schließen",
        body: "Single zählt einen, Double zwei und Triple drei Marks.",
        icon: "marks",
    },
    GameInstruction {
        title: "Offen punkten",
        body: "Weitere Treffer punkten, solange ein Gegner das Feld offen hat.",
        icon: "lock",
    },
    GameInstruction {
        title: "Sieg",
        body: "Schließe alle Ziele und habe mindestens so viele Punkte wie jeder Gegner.",
        icon: "trophy",
    },
];

static COUNTUP_ROUND_CHOICES: [GameOptionChoice; 3] = [
    GameOptionChoice {
        value: GameOptionValue::Integer(5),
        label: "5 Runden",
        description: None,
        description_en: None,
    },
    GameOptionChoice {
        value: GameOptionValue::Integer(8),
        label: "8 Runden",
        description: None,
        description_en: None,
    },
    GameOptionChoice {
        value: GameOptionValue::Integer(10),
        label: "10 Runden",
        description: None,
        description_en: None,
    },
];

static COUNTUP_OPTIONS: [GameOption; 1] = [GameOption {
    key: "rounds",
    label: "Runden",
    kind: "choice",
    default: GameOptionValue::Integer(8),
    choices: &COUNTUP_ROUND_CHOICES,
}];

static COUNTUP_INSTRUCTIONS: [GameInstruction; 3] = [
    GameInstruction {
        title: "Punkte sammeln",
        body: "Jeder Treffer wird direkt zu deinem Konto addiert.",
        icon: "score",
    },
    GameInstruction {
        title: "Drei Darts",
        body: "Eine Aufnahme besteht aus drei Würfen.",
        icon: "darts",
    },
    GameInstruction {
        title: "Vorne gewinnt",
        body: "Nach der gewählten Rundenzahl gewinnt der höchste Score.",
        icon: "trophy",
    },
];

static COUNTUP_METADATA: GameMetadata = GameMetadata {
    slug: "countup",
    ruleset_version: 1,
    title: "Count Up",
    tagline: "Jeder Punkt zählt",
    description: "Sammelt über mehrere Aufnahmen so viele Punkte wie möglich.",
    accent: "#28e7ff",
    accent_secondary: "#176dff",
    visual: "neon-orbit",
    icon: "target",
    artwork: "/static/assets/modes/countup.webp",
    sound_theme: "arena",
    min_players: 1,
    max_players: 8,
    options: &COUNTUP_OPTIONS,
    instructions: &COUNTUP_INSTRUCTIONS,
};

static X01_START_CHOICES: [GameOptionChoice; 3] = [
    GameOptionChoice {
        value: GameOptionValue::Integer(301),
        label: "301",
        description: None,
        description_en: None,
    },
    GameOptionChoice {
        value: GameOptionValue::Integer(501),
        label: "501",
        description: None,
        description_en: None,
    },
    GameOptionChoice {
        value: GameOptionValue::Integer(701),
        label: "701",
        description: None,
        description_en: None,
    },
];

static X01_OUT_CHOICES: [GameOptionChoice; 2] = [
    GameOptionChoice {
        value: GameOptionValue::Text("straight"),
        label: "Straight Out",
        description: Some("Jeder Treffer darf das Spiel exakt auf null beenden."),
        description_en: Some("Any hit may finish the game exactly on zero."),
    },
    GameOptionChoice {
        value: GameOptionValue::Text("double"),
        label: "Double Out",
        description: Some("Der letzte Treffer muss ein Double oder Double Bull sein."),
        description_en: Some("The final hit must be a Double or Double Bull."),
    },
];

static X01_OPTIONS: [GameOption; 2] = [
    GameOption {
        key: "start_score",
        label: "Startpunktzahl",
        kind: "choice",
        default: GameOptionValue::Integer(501),
        choices: &X01_START_CHOICES,
    },
    GameOption {
        key: "out_rule",
        label: "Checkout",
        kind: "choice",
        default: GameOptionValue::Text("straight"),
        choices: &X01_OUT_CHOICES,
    },
];

static X01_INSTRUCTIONS: [GameInstruction; 3] = [
    GameInstruction {
        title: "Herunterspielen",
        body: "Jeder Treffer wird von deinem Restscore abgezogen.",
        icon: "subtract",
    },
    GameInstruction {
        title: "Exakt Null",
        body: "Bringe den Score exakt auf null; bei Double Out muss der letzte Dart ein Double sein.",
        icon: "zero",
    },
    GameInstruction {
        title: "Bust",
        body: "Überwirfst du dich, wird die komplette Aufnahme zurückgesetzt.",
        icon: "bust",
    },
];

static X01_METADATA: GameMetadata = GameMetadata {
    slug: "x01",
    ruleset_version: 1,
    title: "X01",
    tagline: "Runter auf exakt Null",
    description: "Der Turnierklassiker: 301, 501 oder 701 Punkte präzise herunterspielen.",
    accent: "#ffb52b",
    accent_secondary: "#ff3d5f",
    visual: "championship",
    icon: "crown",
    artwork: "/static/assets/modes/x01.webp",
    sound_theme: "championship",
    min_players: 1,
    max_players: 8,
    options: &X01_OPTIONS,
    instructions: &X01_INSTRUCTIONS,
};

static CRICKET_METADATA: GameMetadata = GameMetadata {
    slug: "cricket",
    ruleset_version: 1,
    title: "Cricket",
    tagline: "Schließen und punkten",
    description: "Schließe 15 bis 20 und Bull, während du offene Felder deiner Gegner punktest.",
    accent: "#3dff91",
    accent_secondary: "#11a56a",
    visual: "clubhouse",
    icon: "shield",
    artwork: "/static/assets/modes/cricket.webp",
    sound_theme: "club",
    min_players: 1,
    max_players: 8,
    options: &[],
    instructions: &CRICKET_INSTRUCTIONS,
};

impl GameMode for CricketMode {
    fn metadata(&self) -> &'static GameMetadata {
        &CRICKET_METADATA
    }

    fn initialize(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        for player in &mut state.players {
            player.score = 0;
            player.marks = CRICKET_TARGETS
                .into_iter()
                .map(|target| (target.to_string(), 0))
                .collect();
        }
        Ok(())
    }

    fn apply_throw(
        &self,
        state: &mut RegisteredGameState,
        event: &DartEvent,
    ) -> Result<u32, GameError> {
        let DartEvent::Hit {
            field,
            multiplier,
            label,
            ..
        } = event
        else {
            let player = &state.players[state.current_player_index];
            state.message = format!("{}: kein Cricket-Ziel", player.name);
            return Ok(0);
        };
        if !CRICKET_TARGETS.contains(field) || *multiplier == 0 {
            let player = &state.players[state.current_player_index];
            state.message = format!("{}: kein Cricket-Ziel", player.name);
            return Ok(0);
        }
        let key = field.to_string();
        let player_id = state.players[state.current_player_index].id.clone();
        let before = state.players[state.current_player_index]
            .marks
            .get(&key)
            .copied()
            .unwrap_or_default();
        let total = before.saturating_add(*multiplier);
        let overflow = total.saturating_sub(3);
        state.players[state.current_player_index]
            .marks
            .insert(key.clone(), total.min(3));
        let opponents_open = state.players.iter().any(|player| {
            player.id != player_id && player.marks.get(&key).copied().unwrap_or_default() < 3
        });
        let scored = if opponents_open {
            u32::from(overflow) * u32::from(*field)
        } else {
            0
        };
        let player = &mut state.players[state.current_player_index];
        player.score = player.score.saturating_add(scored);
        state.message = format!("{}: {label}", player.name);
        let closed_all = CRICKET_TARGETS.iter().all(|target| {
            player
                .marks
                .get(&target.to_string())
                .copied()
                .unwrap_or_default()
                >= 3
        });
        let player_score = player.score;
        let leading = state
            .players
            .iter()
            .filter(|candidate| candidate.id != player_id)
            .all(|candidate| player_score >= candidate.score);
        if closed_all && leading {
            let player = &state.players[state.current_player_index];
            state.status = GameStatus::Finished;
            state.winner_id = Some(player.id.clone());
            state.winner_ids = vec![player.id.clone()];
            state.result_type = "individual_win".into();
            state.message = format!("{} gewinnt!", player.name);
        }
        Ok(scored)
    }

    fn overlay(&self, state: &RegisteredGameState) -> Value {
        let Some(player) = state.players.get(state.current_player_index) else {
            return json!({"prompt": "Cricket", "targets": [], "cricket": {"remaining": []}});
        };
        let mut remaining = Vec::new();
        let mut targets = Vec::new();
        for field in CRICKET_TARGETS {
            let marks = player
                .marks
                .get(&field.to_string())
                .copied()
                .unwrap_or_default()
                .min(3);
            let needed = 3 - marks;
            if needed == 0 {
                continue;
            }
            remaining.push(json!({
                "field": field,
                "label": if field == 25 { "BULL".into() } else { field.to_string() },
                "marks": marks,
                "needed": needed,
            }));
            let rings: &[Ring] = if field == 25 {
                &[Ring::SingleBull, Ring::DoubleBull]
            } else {
                &[
                    Ring::SingleInner,
                    Ring::Triple,
                    Ring::SingleOuter,
                    Ring::Double,
                ]
            };
            for ring in rings {
                targets.push(json!({
                    "id": format!("cricket-{field}-{}", ring_name(*ring)),
                    "field": field,
                    "ring": ring,
                    "color": "green",
                    "label": "",
                    "pulse": false,
                }));
            }
        }
        json!({
            "prompt": "Offene Cricket-Ziele",
            "targets": targets,
            "cricket": {"remaining": remaining},
        })
    }
}

static CRICKET_MODE: CricketMode = CricketMode;
static MODES: [&'static dyn GameMode; 1] = [&CRICKET_MODE];

fn mode(slug: &str) -> Result<&'static dyn GameMode, GameError> {
    MODES
        .iter()
        .copied()
        .find(|mode| mode.metadata().slug == slug)
        .ok_or_else(|| GameError::UnknownMode(slug.into()))
}

#[must_use]
pub fn registered_game_metadata() -> Vec<&'static GameMetadata> {
    let mut metadata = vec![&COUNTUP_METADATA, &X01_METADATA];
    metadata.extend(MODES.iter().map(|mode| mode.metadata()));
    metadata.sort_by_key(|metadata| metadata.slug);
    metadata
}

#[must_use]
pub fn game_metadata(slug: &str) -> Option<&'static GameMetadata> {
    match slug {
        "countup" => Some(&COUNTUP_METADATA),
        "x01" => Some(&X01_METADATA),
        _ => mode(slug).ok().map(GameMode::metadata),
    }
}

fn resolve_options(metadata: &GameMetadata, provided: &Value) -> Result<Value, GameError> {
    let supplied = provided
        .as_object()
        .ok_or_else(|| GameError::InvalidOptions("options must be an object".into()))?;
    let mut resolved = Map::new();
    for option in metadata.options {
        resolved.insert(option.key.into(), option.default.as_json());
    }
    for (key, value) in supplied {
        let option = metadata
            .options
            .iter()
            .find(|option| option.key == key)
            .ok_or_else(|| GameError::InvalidOptions(format!("unknown option: {key}")))?;
        let type_matches = matches!(
            (option.default, value),
            (GameOptionValue::Integer(_), Value::Number(number)) if number.is_i64() || number.is_u64()
        ) || matches!(
            (option.default, value),
            (GameOptionValue::Boolean(_), Value::Bool(_))
        ) || matches!(
            (option.default, value),
            (GameOptionValue::Text(_), Value::String(_))
        );
        if !type_matches {
            return Err(GameError::InvalidOptions(format!(
                "invalid value for {}",
                option.key
            )));
        }
        match value {
            Value::Number(number) => {
                let within_limits = number
                    .as_i64()
                    .is_some_and(|value| (-100_000..=100_000).contains(&value))
                    || number.as_u64().is_some_and(|value| value <= 100_000);
                if !within_limits {
                    return Err(GameError::InvalidOptions(format!(
                        "value for {} is outside the supported range",
                        option.key
                    )));
                }
            }
            Value::String(value) if value.len() > 64 => {
                return Err(GameError::InvalidOptions(format!(
                    "value for {} is too long",
                    option.key
                )));
            }
            _ => {}
        }
        if !option.choices.is_empty()
            && !option
                .choices
                .iter()
                .any(|choice| choice.value.as_json() == *value)
        {
            return Err(GameError::InvalidOptions(format!(
                "unsupported value for {}",
                option.key
            )));
        }
        resolved.insert(key.clone(), value.clone());
    }
    Ok(Value::Object(resolved))
}

const fn ring_name(ring: Ring) -> &'static str {
    match ring {
        Ring::SingleInner => "single_inner",
        Ring::SingleOuter => "single_outer",
        Ring::Triple => "triple",
        Ring::Double => "double",
        Ring::SingleBull => "single_bull",
        Ring::DoubleBull => "double_bull",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_exposes_complete_cricket_metadata() {
        let metadata = game_metadata("cricket").expect("Cricket metadata");
        assert_eq!(metadata.ruleset_version, 1);
        assert_eq!(metadata.instructions.len(), 4);
        assert!(metadata.artwork.ends_with("cricket.webp"));
        assert!(game_metadata("missing").is_none());
    }

    #[test]
    fn registry_rejects_unknown_options_without_mutating_game_state() {
        let result = RegisteredGame::new(
            "cricket",
            vec![("ada".into(), "Ada".into())],
            &json!({"surprise": true}),
        );
        assert!(matches!(result, Err(GameError::InvalidOptions(_))));
    }
}
