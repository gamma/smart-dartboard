//! Serialized authoritative runtime.
//!
//! A transition is calculated on a clone, committed through [`Repository`],
//! and only then installed as visible in-memory state. This preserves the
//! product's state when persistence fails between a dart and its broadcast.

use sdb_contracts::{
    ArtTheme, CalibrationSettings, CommandEnvelope, ContractError, DartEvent, DartSource,
    DisplayOverride, EffectDelivery, EffectTarget, ErrorCode, PROTOCOL_VERSION, PlatformEffect,
    PlatformEffectKind, PlayerRef, ProjectorGeometry, RuntimeCommand, RuntimeSettings, SoundOutput,
    SoundStatus, StarterSelection, TeamRef, UiLanguage,
};
use sdb_game_core::{
    CountUpGame, CountUpState, GameError, GameStatus, OutRule, RegisteredGame, RegisteredGameState,
    X01Game, X01State, game_metadata, seed_from_id,
};
use sdb_session_core::{Screen, SessionCore, SessionError, SessionState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuntimeAction {
    CreatePlayer {
        player: PlayerRef,
    },
    StartSession {
        session_id: String,
        players: Vec<PlayerRef>,
        teams: Vec<TeamRef>,
    },
    PrepareGame {
        game_type: String,
        options: serde_json::Value,
    },
    CancelPreparedGame,
    StartPreparedGame {
        game_id: String,
    },
    MarkGamePlaying,
    SelectStarter {
        player_id: String,
        selection: StarterSelection,
    },
    NextGame,
    StartRematch {
        game_id: String,
    },
    AbortGame,
    EndSession,
    CloseSession,
    UpdateCalibration {
        calibration: CalibrationSettings,
    },
    ResetCalibration,
    ReportProjectorGeometry {
        geometry: ProjectorGeometry,
    },
    UpdateSoundSettings {
        enabled: bool,
        output: SoundOutput,
    },
    ReportSoundStatus {
        status: SoundStatus,
    },
    UpdateArtTheme {
        theme: ArtTheme,
    },
    UpdateUiLanguage {
        language: UiLanguage,
    },
    SetCorrectionLock {
        active: bool,
    },
    SoundTest {
        effect_id: String,
    },
    SetDisplayOverride {
        screen: Option<DisplayOverride>,
    },
    StartCountUp {
        players: Vec<(String, String)>,
        rounds: u16,
    },
    StartX01 {
        players: Vec<(String, String)>,
        start_score: u32,
        out_rule: OutRule,
    },
    StartRegistered {
        game_type: String,
        players: Vec<(String, String)>,
        options: serde_json::Value,
        random_seed: u64,
    },
    Dart {
        event: DartEvent,
        source: DartSource,
    },
    CorrectDart {
        action_id: u64,
        replacement: DartEvent,
        source: DartSource,
    },
    DeleteDart {
        action_id: u64,
    },
    GameAction {
        action: String,
        payload: serde_json::Value,
    },
    Continue,
    NextPlayer,
    Undo,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeSnapshot {
    pub revision: u64,
    pub game: Option<RuntimeGame>,
    #[serde(default)]
    pub session: SessionCore,
    #[serde(default)]
    pub settings: RuntimeSettings,
}

/// Read-only state safe to publish to controller and projector clients.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimePublicSnapshot {
    pub revision: u64,
    pub game: Option<RuntimeGameState>,
    pub session: SessionState,
    pub settings: RuntimeSettings,
    #[serde(default)]
    pub effects: Vec<PlatformEffect>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "game_type", content = "game", rename_all = "snake_case")]
pub enum RuntimeGame {
    CountUp(Box<CountUpGame>),
    X01(Box<X01Game>),
    Registered(Box<RegisteredGame>),
}

impl RuntimeGame {
    fn apply_throw(&mut self, event: DartEvent) -> Result<(), GameError> {
        match self {
            Self::CountUp(game) => game.apply_throw(&event).map(|_| ()),
            Self::X01(game) => game.apply_throw(event).map(|_| ()),
            Self::Registered(game) => game.apply_throw(&event).map(|_| ()),
        }
    }

    fn continue_turn(&mut self) -> Result<(), GameError> {
        match self {
            Self::CountUp(game) => game.continue_turn().map(|_| ()),
            Self::X01(game) => game.continue_turn().map(|_| ()),
            Self::Registered(game) => game.continue_turn().map(|_| ()),
        }
    }

    fn next_player(&mut self) -> Result<(), GameError> {
        match self {
            Self::CountUp(game) => game.next_player().map(|_| ()),
            Self::X01(game) => game.next_player().map(|_| ()),
            Self::Registered(game) => game.next_player().map(|_| ()),
        }
    }

    fn undo(&mut self) -> Result<(), GameError> {
        match self {
            Self::CountUp(game) => game.undo().map(|_| ()),
            Self::X01(game) => game.undo().map(|_| ()),
            Self::Registered(game) => game.undo().map(|_| ()),
        }
    }

    fn correct_dart(&mut self, action_id: u64, replacement: DartEvent) -> Result<(), GameError> {
        match self {
            Self::X01(game) => game.correct_throw(action_id, replacement).map(|_| ()),
            Self::Registered(game) => game.correct_throw(action_id, replacement).map(|_| ()),
            Self::CountUp(game) => game.correct_throw(action_id, replacement).map(|_| ()),
        }
    }

    fn delete_dart(&mut self, action_id: u64) -> Result<(), GameError> {
        match self {
            Self::X01(game) => game.delete_throw(action_id).map(|_| ()),
            Self::Registered(game) => game.delete_throw(action_id).map(|_| ()),
            Self::CountUp(game) => game.delete_throw(action_id).map(|_| ()),
        }
    }

    fn handle_action(
        &mut self,
        action: &str,
        payload: &serde_json::Value,
    ) -> Result<(), GameError> {
        match self {
            Self::Registered(game) => game.handle_action(action, payload).map(|_| ()),
            Self::CountUp(_) | Self::X01(_) => Err(GameError::UnsupportedAction(action.into())),
        }
    }

    #[must_use]
    pub fn state(&self) -> RuntimeGameState {
        match self {
            Self::CountUp(game) => RuntimeGameState::CountUp(game.state().clone()),
            Self::X01(game) => RuntimeGameState::X01(game.state().clone()),
            Self::Registered(game) => RuntimeGameState::Registered(game.state().clone()),
        }
    }

    fn is_finished(&self) -> bool {
        match self {
            Self::CountUp(game) => game.state().status == GameStatus::Finished,
            Self::X01(game) => game.state().status == GameStatus::Finished,
            Self::Registered(game) => game.state().status == GameStatus::Finished,
        }
    }

    fn winner_ids(&self) -> &[String] {
        match self {
            Self::CountUp(game) => &game.state().winner_ids,
            Self::X01(game) => &game.state().winner_ids,
            Self::Registered(game) => &game.state().winner_ids,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "game_type", content = "state", rename_all = "snake_case")]
pub enum RuntimeGameState {
    CountUp(CountUpState),
    X01(X01State),
    Registered(RegisteredGameState),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandResult {
    pub command_id: String,
    pub revision: u64,
    pub duplicate: bool,
    pub state: Option<RuntimeGameState>,
    #[serde(default)]
    pub session: SessionState,
    #[serde(default)]
    pub settings: RuntimeSettings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitRequest<'a> {
    pub command_id: &'a str,
    pub runtime_instance_id: &'a str,
    pub previous_revision: u64,
    pub next_revision: u64,
    pub action_json: &'a str,
    pub previous_snapshot_json: &'a str,
    pub snapshot_json: &'a str,
    pub result_json: &'a str,
    pub effects: &'a [PlatformEffect],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitOutcome {
    Committed,
    Duplicate(String),
}

pub trait Repository {
    /// Loads the last committed runtime snapshot.
    ///
    /// # Errors
    ///
    /// Returns a backend-specific diagnostic when reading fails.
    fn load_snapshot(&self) -> Result<Option<String>, String>;

    /// Loads a previously committed result for idempotent command handling.
    ///
    /// # Errors
    ///
    /// Returns a backend-specific diagnostic when reading fails.
    fn load_command_result(&self, command_id: &str) -> Result<Option<String>, String>;

    /// Loads effects that have not yet been acknowledged by their platform target.
    ///
    /// # Errors
    ///
    /// Returns a backend-specific diagnostic when the outbox cannot be read.
    fn load_pending_effects(&self, current_revision: u64) -> Result<Vec<PlatformEffect>, String>;

    /// Marks one platform effect as delivered without creating a game revision.
    ///
    /// # Errors
    ///
    /// Returns a backend-specific diagnostic when the outbox cannot be updated.
    fn acknowledge_effect(&mut self, effect_id: &str) -> Result<bool, String>;

    /// Atomically stores the next revision and its command result.
    ///
    /// # Errors
    ///
    /// Returns a backend-specific diagnostic when the transaction fails or the
    /// expected previous revision no longer matches.
    fn commit(&mut self, request: CommitRequest<'_>) -> Result<CommitOutcome, String>;
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RuntimeError {
    #[error("wrong runtime instance")]
    WrongRuntimeInstance,
    #[error("stale revision: expected {expected}, current {current}")]
    StaleRevision { expected: u64, current: u64 },
    #[error("no game is running")]
    NoGame,
    #[error("game transition failed: {0}")]
    Game(#[from] GameError),
    #[error("persistence failed: {0}")]
    Persistence(String),
    #[error("persisted data is invalid: {0}")]
    InvalidPersistedData(String),
    #[error("session transition failed: {0}")]
    Session(#[from] SessionError),
    #[error("invalid game options: {0}")]
    InvalidGameOptions(String),
    #[error("dart input is paused during operator correction")]
    CorrectionLocked,
}

pub struct Runtime<R> {
    instance_id: String,
    snapshot: RuntimeSnapshot,
    repository: R,
    pending_effects: Vec<PlatformEffect>,
}

impl<R: Repository> Runtime<R> {
    /// Creates a runtime and restores the last committed snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::Persistence`] when the repository cannot be read
    /// and [`RuntimeError::InvalidPersistedData`] for an invalid snapshot.
    pub fn restore(instance_id: impl Into<String>, repository: R) -> Result<Self, RuntimeError> {
        let mut snapshot = repository
            .load_snapshot()
            .map_err(RuntimeError::Persistence)?
            .map(|json| {
                serde_json::from_str(&json)
                    .map_err(|error| RuntimeError::InvalidPersistedData(error.to_string()))
            })
            .transpose()?
            .unwrap_or(RuntimeSnapshot {
                revision: 0,
                game: None,
                session: SessionCore::default(),
                settings: RuntimeSettings::default(),
            });
        snapshot.settings.correction_lock = false;
        snapshot.settings.sound.status = if snapshot.settings.sound.enabled {
            SoundStatus::Starting
        } else {
            SoundStatus::Disabled
        };
        let pending_effects = repository
            .load_pending_effects(snapshot.revision)
            .map_err(RuntimeError::Persistence)?;
        Ok(Self {
            instance_id: instance_id.into(),
            snapshot,
            repository,
            pending_effects,
        })
    }

    #[must_use]
    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }

    #[must_use]
    pub const fn snapshot(&self) -> &RuntimeSnapshot {
        &self.snapshot
    }

    #[must_use]
    pub fn public_snapshot(&self) -> RuntimePublicSnapshot {
        RuntimePublicSnapshot {
            revision: self.snapshot.revision,
            game: self.snapshot.game.as_ref().map(RuntimeGame::state),
            session: self.snapshot.session.state().clone(),
            settings: self.snapshot.settings.clone(),
            effects: self.pending_effects.clone(),
        }
    }

    /// Returns a snapshot suitable for a newly attached or resynchronizing client.
    ///
    /// Discardable effects are live presentation hints and must not be replayed
    /// after a page reload or transport reconnect. Recoverable and durable
    /// effects remain visible until they are acknowledged or expire.
    #[must_use]
    pub fn bootstrap_snapshot(&self) -> RuntimePublicSnapshot {
        let mut snapshot = self.public_snapshot();
        snapshot
            .effects
            .retain(|effect| effect.delivery != EffectDelivery::Discardable);
        snapshot
    }

    #[must_use]
    pub const fn repository(&self) -> &R {
        &self.repository
    }

    #[must_use]
    pub const fn repository_mut(&mut self) -> &mut R {
        &mut self.repository
    }

    /// Acknowledges one committed effect without changing the domain revision.
    ///
    /// # Errors
    ///
    /// Returns a persistence diagnostic if the outbox cannot be updated.
    pub fn acknowledge_effect(
        &mut self,
        effect_id: &str,
        target: EffectTarget,
    ) -> Result<bool, RuntimeError> {
        if !self
            .pending_effects
            .iter()
            .any(|effect| effect.effect_id == effect_id && effect.target == target)
        {
            return Ok(false);
        }
        let acknowledged = self
            .repository
            .acknowledge_effect(effect_id)
            .map_err(RuntimeError::Persistence)?;
        if acknowledged {
            self.pending_effects
                .retain(|effect| effect.effect_id != effect_id);
        }
        Ok(acknowledged)
    }

    /// Validates and applies one transport-neutral command envelope.
    ///
    /// # Errors
    ///
    /// Returns a stable contract error for incompatible protocol versions,
    /// stale clients, unsupported commands, invalid options or persistence
    /// failures.
    pub fn dispatch_envelope(
        &mut self,
        envelope: CommandEnvelope,
    ) -> Result<CommandResult, ContractError> {
        if envelope.protocol_version != PROTOCOL_VERSION {
            return Err(contract_error(
                ErrorCode::IncompatibleProtocol,
                "incompatible protocol version",
                Some(format!(
                    "expected {PROTOCOL_VERSION}, got {}",
                    envelope.protocol_version
                )),
            ));
        }
        let action = command_to_action(envelope.command)?;
        self.dispatch(
            &envelope.runtime_instance_id,
            &envelope.command_id,
            envelope.expected_revision,
            action,
        )
        .map_err(|error| runtime_contract_error(&error))
    }

    /// Applies, commits and then publishes one command result.
    ///
    /// # Errors
    ///
    /// Rejects a wrong runtime instance, stale revision, invalid game action,
    /// serialization error or failed repository transaction.
    pub fn dispatch(
        &mut self,
        runtime_instance_id: &str,
        command_id: &str,
        expected_revision: Option<u64>,
        action: RuntimeAction,
    ) -> Result<CommandResult, RuntimeError> {
        if runtime_instance_id != self.instance_id {
            return Err(RuntimeError::WrongRuntimeInstance);
        }
        if let Some(json) = self
            .repository
            .load_command_result(command_id)
            .map_err(RuntimeError::Persistence)?
        {
            let mut result: CommandResult = serde_json::from_str(&json)
                .map_err(|error| RuntimeError::InvalidPersistedData(error.to_string()))?;
            result.duplicate = true;
            return Ok(result);
        }
        if let Some(expected) = expected_revision
            && expected != self.snapshot.revision
        {
            return Err(RuntimeError::StaleRevision {
                expected,
                current: self.snapshot.revision,
            });
        }

        let action_json = serde_json::to_string(&action)
            .map_err(|error| RuntimeError::InvalidPersistedData(error.to_string()))?;
        let previous_snapshot_json = serde_json::to_string(&self.snapshot)
            .map_err(|error| RuntimeError::InvalidPersistedData(error.to_string()))?;
        let effect_action = action.clone();
        let mut next = self.snapshot.clone();
        apply_action(&mut next, action)?;
        next.revision = self.snapshot.revision + 1;
        let effects = platform_effects(&effect_action, &next);
        let result = CommandResult {
            command_id: command_id.into(),
            revision: next.revision,
            duplicate: false,
            state: next.game.as_ref().map(RuntimeGame::state),
            session: next.session.state().clone(),
            settings: next.settings.clone(),
        };
        let snapshot_json = serde_json::to_string(&next)
            .map_err(|error| RuntimeError::InvalidPersistedData(error.to_string()))?;
        let result_json = serde_json::to_string(&result)
            .map_err(|error| RuntimeError::InvalidPersistedData(error.to_string()))?;

        match self
            .repository
            .commit(CommitRequest {
                command_id,
                runtime_instance_id: &self.instance_id,
                previous_revision: self.snapshot.revision,
                next_revision: next.revision,
                action_json: &action_json,
                previous_snapshot_json: &previous_snapshot_json,
                snapshot_json: &snapshot_json,
                result_json: &result_json,
                effects: &effects,
            })
            .map_err(RuntimeError::Persistence)?
        {
            CommitOutcome::Committed => {
                self.snapshot = next;
                self.pending_effects.retain(|effect| {
                    effect.delivery == EffectDelivery::Durable
                        || effect.revision == self.snapshot.revision
                });
                self.pending_effects.extend(effects);
                Ok(result)
            }
            CommitOutcome::Duplicate(json) => {
                let mut result: CommandResult = serde_json::from_str(&json)
                    .map_err(|error| RuntimeError::InvalidPersistedData(error.to_string()))?;
                result.duplicate = true;
                Ok(result)
            }
        }
    }

    #[must_use]
    pub fn into_repository(self) -> R {
        self.repository
    }
}

fn platform_effects(action: &RuntimeAction, snapshot: &RuntimeSnapshot) -> Vec<PlatformEffect> {
    let (cue, event) = match action {
        RuntimeAction::Dart { event, .. } => {
            (game_effect_cue(snapshot, event), Some(event.clone()))
        }
        RuntimeAction::SoundTest { .. } => ("sound_test".to_owned(), None),
        _ => return Vec::new(),
    };
    let mut effects = Vec::new();
    if let Some(event) = event.as_ref() {
        effects.push(PlatformEffect {
            effect_id: format!("effect:{}:visual:projector", snapshot.revision),
            revision: snapshot.revision,
            target: EffectTarget::Projector,
            delivery: EffectDelivery::Discardable,
            kind: PlatformEffectKind::Visual {
                cue: cue.clone(),
                event: event.clone(),
            },
        });
    }
    if !snapshot.settings.sound.enabled {
        return effects;
    }
    let targets: &[EffectTarget] = match snapshot.settings.sound.output {
        SoundOutput::Controller => &[EffectTarget::Controller],
        SoundOutput::Projector => &[EffectTarget::Projector],
        SoundOutput::Both => &[EffectTarget::Controller, EffectTarget::Projector],
    };
    effects.extend(targets.iter().map(|target| PlatformEffect {
        effect_id: format!(
            "effect:{}:sound:{}",
            snapshot.revision,
            match target {
                EffectTarget::Controller => "controller",
                EffectTarget::Projector => "projector",
            }
        ),
        revision: snapshot.revision,
        target: *target,
        delivery: EffectDelivery::Recoverable,
        kind: PlatformEffectKind::Sound {
            cue: cue.clone(),
            event: event.clone(),
        },
    }));
    effects
}

fn game_effect_cue(snapshot: &RuntimeSnapshot, event: &DartEvent) -> String {
    if matches!(event, DartEvent::Miss { .. }) {
        return "miss".into();
    }
    let Some(RuntimeGame::Registered(game)) = snapshot.game.as_ref() else {
        return "hit".into();
    };
    game.state()
        .mode_state
        .get("last_effect")
        .and_then(serde_json::Value::as_str)
        .filter(|cue| !cue.is_empty())
        .unwrap_or("hit")
        .to_owned()
}

#[allow(clippy::too_many_lines)] // One exhaustive match keeps the public protocol mapping visible.
fn command_to_action(command: RuntimeCommand) -> Result<RuntimeAction, ContractError> {
    match command {
        RuntimeCommand::CreatePlayer { player } => Ok(RuntimeAction::CreatePlayer { player }),
        RuntimeCommand::StartSession {
            session_id,
            players,
            teams,
        } => Ok(RuntimeAction::StartSession {
            session_id,
            players,
            teams,
        }),
        RuntimeCommand::PrepareGame { game_type, options } => {
            Ok(RuntimeAction::PrepareGame { game_type, options })
        }
        RuntimeCommand::CancelPreparedGame => Ok(RuntimeAction::CancelPreparedGame),
        RuntimeCommand::StartPreparedGame { game_id } => {
            Ok(RuntimeAction::StartPreparedGame { game_id })
        }
        RuntimeCommand::MarkGamePlaying => Ok(RuntimeAction::MarkGamePlaying),
        RuntimeCommand::SelectStarter {
            player_id,
            selection,
        } => Ok(RuntimeAction::SelectStarter {
            player_id,
            selection,
        }),
        RuntimeCommand::NextGame => Ok(RuntimeAction::NextGame),
        RuntimeCommand::StartRematch { game_id } => Ok(RuntimeAction::StartRematch { game_id }),
        RuntimeCommand::EndSession => Ok(RuntimeAction::EndSession),
        RuntimeCommand::CloseSession => Ok(RuntimeAction::CloseSession),
        RuntimeCommand::UpdateCalibration { calibration } => {
            validate_calibration(&calibration)?;
            Ok(RuntimeAction::UpdateCalibration { calibration })
        }
        RuntimeCommand::ResetCalibration => Ok(RuntimeAction::ResetCalibration),
        RuntimeCommand::ReportProjectorGeometry { geometry } => {
            if !(320..=16_384).contains(&geometry.width)
                || !(240..=16_384).contains(&geometry.height)
            {
                return Err(invalid_command(
                    "projector geometry is outside the supported range",
                ));
            }
            Ok(RuntimeAction::ReportProjectorGeometry { geometry })
        }
        RuntimeCommand::UpdateSoundSettings { enabled, output } => {
            Ok(RuntimeAction::UpdateSoundSettings { enabled, output })
        }
        RuntimeCommand::ReportSoundStatus { status } => {
            if !matches!(
                status,
                SoundStatus::Ready | SoundStatus::Blocked | SoundStatus::Unavailable
            ) {
                return Err(invalid_command("invalid reported sound status"));
            }
            Ok(RuntimeAction::ReportSoundStatus { status })
        }
        RuntimeCommand::UpdateArtTheme { theme } => Ok(RuntimeAction::UpdateArtTheme { theme }),
        RuntimeCommand::UpdateUiLanguage { language } => {
            Ok(RuntimeAction::UpdateUiLanguage { language })
        }
        RuntimeCommand::SetCorrectionLock { active } => {
            Ok(RuntimeAction::SetCorrectionLock { active })
        }
        RuntimeCommand::SoundTest { effect_id } => {
            if effect_id.is_empty() || effect_id.len() > 128 {
                return Err(invalid_command(
                    "sound test effect_id must contain 1 to 128 bytes",
                ));
            }
            Ok(RuntimeAction::SoundTest { effect_id })
        }
        RuntimeCommand::SetDisplayOverride { screen } => {
            Ok(RuntimeAction::SetDisplayOverride { screen })
        }
        RuntimeCommand::IngestDart { event, source } => Ok(RuntimeAction::Dart { event, source }),
        RuntimeCommand::CorrectDart {
            action_id,
            replacement,
            source,
        } => Ok(RuntimeAction::CorrectDart {
            action_id,
            replacement,
            source,
        }),
        RuntimeCommand::DeleteDart { action_id } => Ok(RuntimeAction::DeleteDart { action_id }),
        RuntimeCommand::StartGame {
            game_type,
            player_ids,
            options,
        } => {
            let random_seed = seed_from_id(&format!(
                "direct:{game_type}:{}:{options}",
                player_ids.join("\u{1f}")
            ));
            let players = player_ids.into_iter().map(|id| (id.clone(), id)).collect();
            match game_type.as_str() {
                "countup" => {
                    let rounds = option_u64(&options, "rounds", 8)?;
                    let rounds = u16::try_from(rounds).map_err(|_| {
                        invalid_command("countup rounds must fit into an unsigned 16-bit integer")
                    })?;
                    Ok(RuntimeAction::StartCountUp { players, rounds })
                }
                "x01" => {
                    let start_score = option_u64(&options, "start_score", 501)?;
                    let start_score = u32::try_from(start_score).map_err(|_| {
                        invalid_command("X01 start_score must fit into an unsigned 32-bit integer")
                    })?;
                    let out_rule = match options
                        .get("out_rule")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("straight")
                    {
                        "straight" => OutRule::Straight,
                        "double" => OutRule::Double,
                        _ => {
                            return Err(invalid_command("X01 out_rule must be straight or double"));
                        }
                    };
                    Ok(RuntimeAction::StartX01 {
                        players,
                        start_score,
                        out_rule,
                    })
                }
                _ if game_metadata(&game_type).is_some() => Ok(RuntimeAction::StartRegistered {
                    game_type,
                    players,
                    options,
                    random_seed,
                }),
                _ => Err(invalid_command("unsupported game type")),
            }
        }
        RuntimeCommand::ContinueTurn => Ok(RuntimeAction::Continue),
        RuntimeCommand::NextPlayer => Ok(RuntimeAction::NextPlayer),
        RuntimeCommand::Undo => Ok(RuntimeAction::Undo),
        RuntimeCommand::AbortGame => Ok(RuntimeAction::AbortGame),
        RuntimeCommand::GameAction { action, payload } => {
            Ok(RuntimeAction::GameAction { action, payload })
        }
    }
}

fn validate_calibration(calibration: &CalibrationSettings) -> Result<(), ContractError> {
    let scalar_values = [
        calibration.scale,
        calibration.offset_x,
        calibration.offset_y,
    ];
    if scalar_values.iter().any(|value| !value.is_finite())
        || !(0.5..=2.0).contains(&calibration.scale)
        || !(-1.0..=1.0).contains(&calibration.offset_x)
        || !(-1.0..=1.0).contains(&calibration.offset_y)
        || calibration
            .corners
            .iter()
            .any(|point| !point.x.is_finite() || !point.y.is_finite())
        || calibration
            .corners
            .iter()
            .any(|point| !(0.0..=1.0).contains(&point.x) || !(0.0..=1.0).contains(&point.y))
    {
        return Err(invalid_command(
            "calibration is outside the supported range",
        ));
    }
    Ok(())
}

fn option_u64(options: &serde_json::Value, name: &str, default: u64) -> Result<u64, ContractError> {
    options.get(name).map_or(Ok(default), |value| {
        value
            .as_u64()
            .ok_or_else(|| invalid_command(&format!("{name} must be an unsigned integer")))
    })
}

fn invalid_command(message: &str) -> ContractError {
    contract_error(ErrorCode::InvalidCommand, message, None)
}

fn contract_error(code: ErrorCode, message: &str, details: Option<String>) -> ContractError {
    ContractError {
        code,
        message: message.into(),
        details,
    }
}

fn runtime_contract_error(error: &RuntimeError) -> ContractError {
    let code = match error {
        RuntimeError::WrongRuntimeInstance => ErrorCode::WrongRuntimeInstance,
        RuntimeError::StaleRevision { .. } => ErrorCode::StaleRevision,
        RuntimeError::Persistence(_) => ErrorCode::PersistenceFailed,
        RuntimeError::CorrectionLocked => ErrorCode::Forbidden,
        RuntimeError::NoGame
        | RuntimeError::Game(_)
        | RuntimeError::InvalidPersistedData(_)
        | RuntimeError::Session(_)
        | RuntimeError::InvalidGameOptions(_) => ErrorCode::InvalidCommand,
    };
    contract_error(code, &error.to_string(), None)
}

#[allow(clippy::too_many_lines)] // One exhaustive match keeps every atomic runtime transition visible.
fn apply_action(snapshot: &mut RuntimeSnapshot, action: RuntimeAction) -> Result<(), RuntimeError> {
    match action {
        RuntimeAction::CreatePlayer { player } => {
            if !valid_player_profile(&player) {
                return Err(SessionError::InvalidPlayer.into());
            }
        }
        RuntimeAction::StartSession {
            session_id,
            players,
            teams,
        } => {
            snapshot
                .session
                .start_session_with_teams(session_id, players, teams)?;
            snapshot.game = None;
            snapshot.settings.display_override = None;
        }
        RuntimeAction::PrepareGame { game_type, options } => {
            let format = game_metadata(&game_type)
                .map_or(sdb_contracts::GameFormat::Individual, |metadata| {
                    metadata.format
                });
            snapshot
                .session
                .prepare_game_with_format(game_type, options, format)?;
        }
        RuntimeAction::CancelPreparedGame => {
            snapshot.session.cancel_prepared_game()?;
        }
        RuntimeAction::StartPreparedGame { game_id } => {
            start_prepared_game(snapshot, game_id, false)?;
        }
        RuntimeAction::MarkGamePlaying => {
            snapshot.session.mark_playing()?;
        }
        RuntimeAction::SelectStarter {
            player_id,
            selection,
        } => {
            snapshot.session.select_starter(&player_id, selection)?;
        }
        RuntimeAction::NextGame => {
            snapshot.session.next_game()?;
            snapshot.game = None;
        }
        RuntimeAction::StartRematch { game_id } => {
            start_prepared_game(snapshot, game_id, true)?;
        }
        RuntimeAction::AbortGame => {
            if snapshot.session.state().session_id.is_some() {
                snapshot.session.abort_game()?;
            }
            snapshot.game = None;
        }
        RuntimeAction::EndSession => {
            snapshot.session.end_session()?;
        }
        RuntimeAction::CloseSession => {
            snapshot.session.close_session();
            snapshot.game = None;
        }
        RuntimeAction::UpdateCalibration { calibration } => {
            snapshot.settings.calibration = calibration;
        }
        RuntimeAction::ResetCalibration => {
            let width = f64::from(snapshot.settings.projector_geometry.width);
            let height = f64::from(snapshot.settings.projector_geometry.height);
            let side = width.min(height) * 0.9;
            let half_x = side / width / 2.0;
            let half_y = side / height / 2.0;
            snapshot.settings.calibration = CalibrationSettings {
                corners: [
                    sdb_contracts::CalibrationPoint {
                        x: 0.5 - half_x,
                        y: 0.5 - half_y,
                    },
                    sdb_contracts::CalibrationPoint {
                        x: 0.5 + half_x,
                        y: 0.5 - half_y,
                    },
                    sdb_contracts::CalibrationPoint {
                        x: 0.5 + half_x,
                        y: 0.5 + half_y,
                    },
                    sdb_contracts::CalibrationPoint {
                        x: 0.5 - half_x,
                        y: 0.5 + half_y,
                    },
                ],
                scale: 1.0,
                offset_x: 0.0,
                offset_y: 0.0,
            };
        }
        RuntimeAction::ReportProjectorGeometry { geometry } => {
            snapshot.settings.projector_geometry = geometry;
        }
        RuntimeAction::UpdateSoundSettings { enabled, output } => {
            snapshot.settings.sound.enabled = enabled;
            snapshot.settings.sound.output = output;
            snapshot.settings.sound.status = if enabled {
                SoundStatus::Starting
            } else {
                SoundStatus::Disabled
            };
        }
        RuntimeAction::ReportSoundStatus { status } => {
            snapshot.settings.sound.status = status;
        }
        RuntimeAction::UpdateArtTheme { theme } => snapshot.settings.art_theme = theme,
        RuntimeAction::UpdateUiLanguage { language } => snapshot.settings.ui_language = language,
        RuntimeAction::SetCorrectionLock { active } => {
            snapshot.settings.correction_lock = active;
        }
        RuntimeAction::SoundTest { effect_id } => {
            snapshot.settings.sound_test_id = Some(effect_id);
        }
        RuntimeAction::SetDisplayOverride { screen } => {
            snapshot.settings.display_override = screen;
        }
        direct_action @ (RuntimeAction::StartCountUp { .. }
        | RuntimeAction::StartX01 { .. }
        | RuntimeAction::StartRegistered { .. }) => start_direct_game(snapshot, direct_action)?,
        RuntimeAction::Dart { event, source } => {
            if snapshot.settings.correction_lock && source != DartSource::ManualCorrection {
                return Err(RuntimeError::CorrectionLocked);
            }
            snapshot
                .game
                .as_mut()
                .ok_or(RuntimeError::NoGame)?
                .apply_throw(event)?;
            sync_finished_game(snapshot)?;
        }
        RuntimeAction::CorrectDart {
            action_id,
            replacement,
            ..
        } => {
            let game = snapshot.game.as_mut().ok_or(RuntimeError::NoGame)?;
            game.correct_dart(action_id, replacement)?;
            sync_edited_game(snapshot)?;
        }
        RuntimeAction::DeleteDart { action_id } => {
            let game = snapshot.game.as_mut().ok_or(RuntimeError::NoGame)?;
            game.delete_dart(action_id)?;
            sync_edited_game(snapshot)?;
        }
        RuntimeAction::GameAction { action, payload } => {
            snapshot
                .game
                .as_mut()
                .ok_or(RuntimeError::NoGame)?
                .handle_action(&action, &payload)?;
            sync_finished_game(snapshot)?;
        }
        RuntimeAction::Continue => {
            apply_player_boundary(snapshot, false)?;
        }
        RuntimeAction::NextPlayer => {
            apply_player_boundary(snapshot, true)?;
        }
        RuntimeAction::Undo => {
            let game = snapshot.game.as_mut().ok_or(RuntimeError::NoGame)?;
            let was_finished = game.is_finished();
            game.undo()?;
            if was_finished
                && !game.is_finished()
                && snapshot.session.state().screen == Screen::GameResult
            {
                snapshot.session.reopen_game()?;
            }
        }
    }
    Ok(())
}

fn valid_player_profile(player: &PlayerRef) -> bool {
    player.id.len() <= 128
        && !player.id.is_empty()
        && !player.name.trim().is_empty()
        && player.name.chars().count() <= 32
        && !player.avatar.is_empty()
        && player.avatar.len() <= 32
        && player.color.len() == 7
        && player.color.starts_with('#')
        && player.color[1..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
}

fn apply_player_boundary(
    snapshot: &mut RuntimeSnapshot,
    next_player: bool,
) -> Result<(), RuntimeError> {
    let game = snapshot.game.as_mut().ok_or(RuntimeError::NoGame)?;
    if next_player {
        game.next_player()?;
    } else {
        game.continue_turn()?;
    }
    sync_finished_game(snapshot)
}

fn start_direct_game(
    snapshot: &mut RuntimeSnapshot,
    action: RuntimeAction,
) -> Result<(), RuntimeError> {
    snapshot.game = Some(match action {
        RuntimeAction::StartCountUp { players, rounds } => {
            RuntimeGame::CountUp(Box::new(CountUpGame::new(players, rounds)?))
        }
        RuntimeAction::StartX01 {
            players,
            start_score,
            out_rule,
        } => RuntimeGame::X01(Box::new(X01Game::new(players, start_score, out_rule)?)),
        RuntimeAction::StartRegistered {
            game_type,
            players,
            options,
            random_seed,
        } => RuntimeGame::Registered(Box::new(RegisteredGame::new_seeded(
            &game_type,
            players,
            &options,
            random_seed,
        )?)),
        _ => unreachable!("start_direct_game only accepts direct game actions"),
    });
    Ok(())
}

fn start_prepared_game(
    snapshot: &mut RuntimeSnapshot,
    game_id: String,
    rematch: bool,
) -> Result<(), RuntimeError> {
    let random_seed = seed_from_id(&game_id);
    let prepared = snapshot
        .session
        .state()
        .prepared_game
        .clone()
        .ok_or(SessionError::NoPreparedGame)?;
    let ordered = if rematch {
        snapshot.session.start_rematch(game_id)?
    } else {
        snapshot.session.start_game(game_id)?
    };
    snapshot.game = Some(game_from_options(
        &prepared.game_type,
        ordered,
        &prepared.options,
        random_seed,
    )?);
    Ok(())
}

fn game_from_options(
    game_type: &str,
    players: Vec<PlayerRef>,
    options: &serde_json::Value,
    random_seed: u64,
) -> Result<RuntimeGame, RuntimeError> {
    match game_type {
        "countup" => {
            let players = players
                .into_iter()
                .map(|player| (player.id, player.name))
                .collect();
            let rounds = options
                .get("rounds")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(8);
            let rounds = u16::try_from(rounds).map_err(|_| {
                RuntimeError::InvalidGameOptions("countup rounds are out of range".into())
            })?;
            Ok(RuntimeGame::CountUp(Box::new(CountUpGame::new(
                players, rounds,
            )?)))
        }
        "x01" => {
            let players = players
                .into_iter()
                .map(|player| (player.id, player.name))
                .collect();
            let start_score = options
                .get("start_score")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(501);
            let start_score = u32::try_from(start_score).map_err(|_| {
                RuntimeError::InvalidGameOptions("X01 start score is out of range".into())
            })?;
            let out_rule = match options
                .get("out_rule")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("straight")
            {
                "straight" => OutRule::Straight,
                "double" => OutRule::Double,
                _ => {
                    return Err(RuntimeError::InvalidGameOptions(
                        "X01 out rule must be straight or double".into(),
                    ));
                }
            };
            Ok(RuntimeGame::X01(Box::new(X01Game::new(
                players,
                start_score,
                out_rule,
            )?)))
        }
        _ if game_metadata(game_type).is_some() => Ok(RuntimeGame::Registered(Box::new(
            RegisteredGame::new_seeded_with_players(game_type, players, options, random_seed)?,
        ))),
        _ => Err(RuntimeError::InvalidGameOptions(format!(
            "unsupported game type: {game_type}"
        ))),
    }
}

fn sync_finished_game(snapshot: &mut RuntimeSnapshot) -> Result<(), RuntimeError> {
    let Some(game) = snapshot.game.as_ref() else {
        return Ok(());
    };
    if game.is_finished()
        && matches!(
            snapshot.session.state().screen,
            Screen::Countdown | Screen::Playing
        )
    {
        snapshot.session.complete_game(game.winner_ids())?;
    }
    Ok(())
}

fn sync_edited_game(snapshot: &mut RuntimeSnapshot) -> Result<(), RuntimeError> {
    if snapshot.session.state().screen == Screen::GameResult {
        snapshot.session.reopen_game()?;
    }
    sync_finished_game(snapshot)
}

#[derive(Debug, Default)]
pub struct MemoryRepository {
    snapshot: Option<String>,
    results: HashMap<String, String>,
    fail_next_commit: bool,
    effects: HashMap<String, PlatformEffect>,
}

impl MemoryRepository {
    pub fn fail_next_commit(&mut self) {
        self.fail_next_commit = true;
    }
}

impl Repository for MemoryRepository {
    fn load_snapshot(&self) -> Result<Option<String>, String> {
        Ok(self.snapshot.clone())
    }

    fn load_command_result(&self, command_id: &str) -> Result<Option<String>, String> {
        Ok(self.results.get(command_id).cloned())
    }

    fn load_pending_effects(&self, current_revision: u64) -> Result<Vec<PlatformEffect>, String> {
        let mut effects: Vec<_> = self
            .effects
            .values()
            .filter(|effect| {
                effect.delivery == EffectDelivery::Durable
                    || (effect.delivery == EffectDelivery::Recoverable
                        && effect.revision == current_revision)
            })
            .cloned()
            .collect();
        effects.sort_by(|left, right| left.effect_id.cmp(&right.effect_id));
        Ok(effects)
    }

    fn acknowledge_effect(&mut self, effect_id: &str) -> Result<bool, String> {
        Ok(self.effects.remove(effect_id).is_some())
    }

    fn commit(&mut self, request: CommitRequest<'_>) -> Result<CommitOutcome, String> {
        if self.fail_next_commit {
            self.fail_next_commit = false;
            return Err("injected commit failure".into());
        }
        if let Some(result) = self.results.get(request.command_id) {
            return Ok(CommitOutcome::Duplicate(result.clone()));
        }
        self.snapshot = Some(request.snapshot_json.into());
        self.results
            .insert(request.command_id.into(), request.result_json.into());
        self.effects.retain(|_, effect| {
            effect.delivery == EffectDelivery::Durable || effect.revision == request.next_revision
        });
        self.effects.extend(
            request
                .effects
                .iter()
                .cloned()
                .map(|effect| (effect.effect_id.clone(), effect)),
        );
        Ok(CommitOutcome::Committed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdb_contracts::Ring;

    fn players() -> Vec<(String, String)> {
        vec![("ada".into(), "Ada".into())]
    }

    fn session_players() -> Vec<PlayerRef> {
        vec![
            PlayerRef {
                id: "ada".into(),
                name: "Ada".into(),
                avatar: "nova".into(),
                color: "#ff00aa".into(),
                team_id: None,
            },
            PlayerRef {
                id: "bob".into(),
                name: "Bob".into(),
                avatar: "comet".into(),
                color: "#28e7ff".into(),
                team_id: None,
            },
        ]
    }

    #[test]
    fn failed_commit_does_not_publish_state() {
        let repository = MemoryRepository::default();
        let mut runtime = Runtime::restore("runtime", repository).expect("runtime");
        runtime.repository.fail_next_commit();
        let error = runtime
            .dispatch(
                "runtime",
                "start-1",
                Some(0),
                RuntimeAction::StartCountUp {
                    players: players(),
                    rounds: 5,
                },
            )
            .expect_err("commit must fail");
        assert!(matches!(error, RuntimeError::Persistence(_)));
        assert_eq!(runtime.snapshot.revision, 0);
        assert!(runtime.snapshot.game.is_none());
    }

    #[test]
    fn duplicate_command_returns_original_revision() {
        let repository = MemoryRepository::default();
        let mut runtime = Runtime::restore("runtime", repository).expect("runtime");
        let first = runtime
            .dispatch(
                "runtime",
                "start-1",
                Some(0),
                RuntimeAction::StartCountUp {
                    players: players(),
                    rounds: 5,
                },
            )
            .expect("first command");
        let duplicate = runtime
            .dispatch("runtime", "start-1", None, RuntimeAction::Undo)
            .expect("duplicate command");
        assert_eq!(first.revision, 1);
        assert_eq!(duplicate.revision, 1);
        assert!(duplicate.duplicate);
        assert_eq!(runtime.snapshot.revision, 1);
    }

    #[test]
    fn x01_uses_the_same_atomic_runtime_boundary() {
        let repository = MemoryRepository::default();
        let mut runtime = Runtime::restore("runtime", repository).expect("runtime");
        runtime
            .dispatch(
                "runtime",
                "start-x01",
                Some(0),
                RuntimeAction::StartX01 {
                    players: players(),
                    start_score: 40,
                    out_rule: OutRule::Double,
                },
            )
            .expect("start X01");
        let result = runtime
            .dispatch(
                "runtime",
                "checkout",
                Some(1),
                RuntimeAction::Dart {
                    event: DartEvent::Hit {
                        seq: 1,
                        field: 20,
                        ring: Ring::Double,
                        multiplier: 2,
                        label: "D20".into(),
                        score: 40,
                    },
                    source: DartSource::Board,
                },
            )
            .expect("checkout");

        let RuntimeGameState::X01(state) = result.state.expect("state") else {
            panic!("wrong game type");
        };
        assert_eq!(result.revision, 2);
        assert_eq!(state.winner_id.as_deref(), Some("ada"));
        assert_eq!(state.players[0].score, 0);
    }

    #[test]
    fn prepared_cooperative_mode_materializes_the_shared_team_in_game_state() {
        let repository = MemoryRepository::default();
        let mut runtime = Runtime::restore("runtime", repository).expect("runtime");
        runtime
            .dispatch(
                "runtime",
                "session",
                None,
                RuntimeAction::StartSession {
                    session_id: "session".into(),
                    players: session_players(),
                    teams: Vec::new(),
                },
            )
            .expect("session");
        runtime
            .dispatch(
                "runtime",
                "prepare",
                None,
                RuntimeAction::PrepareGame {
                    game_type: "boss_fight".into(),
                    options: serde_json::json!({}),
                },
            )
            .expect("prepare");
        assert_eq!(
            runtime.snapshot().session.state().active_game_teams[0].player_ids,
            ["ada", "bob"]
        );
        runtime
            .dispatch(
                "runtime",
                "start",
                None,
                RuntimeAction::StartPreparedGame {
                    game_id: "game".into(),
                },
            )
            .expect("start");
        let RuntimeGame::Registered(game) = runtime.snapshot().game.as_ref().expect("game") else {
            panic!("registered game expected");
        };
        assert!(
            game.state()
                .players
                .iter()
                .all(|player| player.team_id.as_deref() == Some("coop"))
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)] // Keeps the full win/correct/re-win/delete flow together.
    fn correcting_and_deleting_checkout_synchronizes_session_points() {
        let repository = MemoryRepository::default();
        let mut runtime = Runtime::restore("runtime", repository).expect("runtime");
        runtime
            .dispatch(
                "runtime",
                "session",
                None,
                RuntimeAction::StartSession {
                    session_id: "session".into(),
                    players: vec![session_players()[0].clone()],
                    teams: Vec::new(),
                },
            )
            .expect("session");
        runtime
            .dispatch(
                "runtime",
                "prepare",
                None,
                RuntimeAction::PrepareGame {
                    game_type: "x01".into(),
                    options: serde_json::json!({"start_score": 40, "out_rule": "double"}),
                },
            )
            .expect("prepare");
        runtime
            .dispatch(
                "runtime",
                "start",
                None,
                RuntimeAction::StartPreparedGame {
                    game_id: "game".into(),
                },
            )
            .expect("start");
        runtime
            .dispatch("runtime", "playing", None, RuntimeAction::MarkGamePlaying)
            .expect("playing");
        runtime
            .dispatch(
                "runtime",
                "checkout",
                None,
                RuntimeAction::Dart {
                    event: DartEvent::Hit {
                        seq: 1,
                        field: 20,
                        ring: Ring::Double,
                        multiplier: 2,
                        label: "D20".into(),
                        score: 40,
                    },
                    source: DartSource::Board,
                },
            )
            .expect("checkout");
        assert_eq!(runtime.snapshot.session.state().screen, Screen::GameResult);
        assert_eq!(
            runtime.snapshot.session.state().standings[0].session_points,
            3
        );

        runtime
            .dispatch(
                "runtime",
                "correct",
                None,
                RuntimeAction::CorrectDart {
                    action_id: 1,
                    replacement: DartEvent::Hit {
                        seq: 999,
                        field: 20,
                        ring: Ring::SingleInner,
                        multiplier: 1,
                        label: "S20".into(),
                        score: 20,
                    },
                    source: DartSource::ManualCorrection,
                },
            )
            .expect("correction");
        assert_eq!(runtime.snapshot.session.state().screen, Screen::Playing);
        assert_eq!(
            runtime.snapshot.session.state().standings[0].session_points,
            0
        );
        let Some(RuntimeGame::X01(game)) = &runtime.snapshot.game else {
            panic!("wrong game");
        };
        assert_eq!(game.state().players[0].score, 20);
        assert_eq!(game.dart_actions()[0].1.seq(), 1);

        runtime
            .dispatch(
                "runtime",
                "correct-back",
                None,
                RuntimeAction::CorrectDart {
                    action_id: 1,
                    replacement: DartEvent::Hit {
                        seq: 1000,
                        field: 20,
                        ring: Ring::Double,
                        multiplier: 2,
                        label: "D20".into(),
                        score: 40,
                    },
                    source: DartSource::ManualCorrection,
                },
            )
            .expect("correction back");
        assert_eq!(runtime.snapshot.session.state().screen, Screen::GameResult);
        assert_eq!(
            runtime.snapshot.session.state().standings[0].session_points,
            3
        );

        runtime
            .dispatch(
                "runtime",
                "delete",
                None,
                RuntimeAction::DeleteDart { action_id: 1 },
            )
            .expect("delete checkout");
        assert_eq!(runtime.snapshot.session.state().screen, Screen::Playing);
        assert_eq!(
            runtime.snapshot.session.state().standings[0].session_points,
            0
        );
    }

    #[test]
    fn command_envelope_validates_protocol_and_dispatches_x01() {
        let repository = MemoryRepository::default();
        let mut runtime = Runtime::restore("runtime", repository).expect("runtime");
        let incompatible = runtime
            .dispatch_envelope(CommandEnvelope {
                protocol_version: PROTOCOL_VERSION + 1,
                command_id: "bad-version".into(),
                runtime_instance_id: "runtime".into(),
                expected_revision: Some(0),
                command: RuntimeCommand::Undo,
            })
            .expect_err("protocol must be rejected");
        assert_eq!(incompatible.code, ErrorCode::IncompatibleProtocol);

        let started = runtime
            .dispatch_envelope(CommandEnvelope {
                protocol_version: PROTOCOL_VERSION,
                command_id: "start".into(),
                runtime_instance_id: "runtime".into(),
                expected_revision: Some(0),
                command: RuntimeCommand::StartGame {
                    game_type: "x01".into(),
                    player_ids: vec!["ada".into()],
                    options: serde_json::json!({
                        "start_score": 301,
                        "out_rule": "double"
                    }),
                },
            })
            .expect("start X01");
        assert_eq!(started.revision, 1);
        let Some(RuntimeGameState::X01(state)) = started.state else {
            panic!("wrong game type");
        };
        assert_eq!(state.start_score, 301);
        assert_eq!(state.out_rule, OutRule::Double);
    }

    #[test]
    fn public_snapshot_omits_the_internal_replay_timeline() {
        let repository = MemoryRepository::default();
        let mut runtime = Runtime::restore("runtime", repository).expect("runtime");
        runtime
            .dispatch(
                "runtime",
                "start",
                Some(0),
                RuntimeAction::StartCountUp {
                    players: vec![("ada".into(), "Ada".into())],
                    rounds: 5,
                },
            )
            .expect("start");

        let public = serde_json::to_value(runtime.public_snapshot()).expect("public snapshot");
        assert_eq!(public["revision"], 1);
        assert_eq!(public["game"]["game_type"], "count_up");
        assert_eq!(public["game"]["state"]["players"][0]["id"], "ada");
        assert!(public["game"].get("history").is_none());
        assert!(public["game"].get("actions").is_none());
        assert!(public["game"].get("initial_state").is_none());
    }

    #[test]
    fn next_player_command_ends_a_running_partial_visit_and_is_undoable() {
        let repository = MemoryRepository::default();
        let mut runtime = Runtime::restore("runtime", repository).expect("runtime");
        runtime
            .dispatch_envelope(CommandEnvelope {
                protocol_version: PROTOCOL_VERSION,
                command_id: "start-countup".into(),
                runtime_instance_id: "runtime".into(),
                expected_revision: Some(0),
                command: RuntimeCommand::StartGame {
                    game_type: "countup".into(),
                    player_ids: vec!["ada".into(), "bob".into()],
                    options: serde_json::json!({"rounds": 5}),
                },
            })
            .expect("start CountUp");
        runtime
            .dispatch_envelope(CommandEnvelope {
                protocol_version: PROTOCOL_VERSION,
                command_id: "partial-dart".into(),
                runtime_instance_id: "runtime".into(),
                expected_revision: Some(1),
                command: RuntimeCommand::IngestDart {
                    event: DartEvent::Hit {
                        seq: 1,
                        field: 20,
                        ring: Ring::Triple,
                        multiplier: 3,
                        label: "T20".into(),
                        score: 60,
                    },
                    source: DartSource::Board,
                },
            })
            .expect("partial visit");

        let continue_error = runtime
            .dispatch_envelope(CommandEnvelope {
                protocol_version: PROTOCOL_VERSION,
                command_id: "continue-running-visit".into(),
                runtime_instance_id: "runtime".into(),
                expected_revision: Some(2),
                command: RuntimeCommand::ContinueTurn,
            })
            .expect_err("continue must not skip a running visit");
        assert_eq!(continue_error.code, ErrorCode::InvalidCommand);
        assert_eq!(runtime.snapshot().revision, 2);

        let skipped = runtime
            .dispatch_envelope(CommandEnvelope {
                protocol_version: PROTOCOL_VERSION,
                command_id: "next-player".into(),
                runtime_instance_id: "runtime".into(),
                expected_revision: Some(2),
                command: RuntimeCommand::NextPlayer,
            })
            .expect("next player");
        let Some(RuntimeGameState::CountUp(state)) = skipped.state else {
            panic!("wrong game type");
        };
        assert_eq!(state.current_player_index, 1);
        assert_eq!(state.players[0].score, 60);
        assert_eq!(state.darts_in_turn, 0);

        let restored = runtime
            .dispatch_envelope(CommandEnvelope {
                protocol_version: PROTOCOL_VERSION,
                command_id: "undo-next-player".into(),
                runtime_instance_id: "runtime".into(),
                expected_revision: Some(3),
                command: RuntimeCommand::Undo,
            })
            .expect("undo next player");
        let Some(RuntimeGameState::CountUp(state)) = restored.state else {
            panic!("wrong game type");
        };
        assert_eq!(state.current_player_index, 0);
        assert_eq!(state.darts_in_turn, 1);
        assert_eq!(state.players[0].score, 60);
    }

    #[test]
    fn command_envelope_starts_a_registered_cricket_game() {
        let repository = MemoryRepository::default();
        let mut runtime = Runtime::restore("runtime", repository).expect("runtime");
        let started = runtime
            .dispatch_envelope(CommandEnvelope {
                protocol_version: PROTOCOL_VERSION,
                command_id: "start-cricket".into(),
                runtime_instance_id: "runtime".into(),
                expected_revision: Some(0),
                command: RuntimeCommand::StartGame {
                    game_type: "cricket".into(),
                    player_ids: vec!["ada".into(), "bob".into()],
                    options: serde_json::json!({}),
                },
            })
            .expect("start Cricket");
        let Some(RuntimeGameState::Registered(state)) = started.state else {
            panic!("wrong game type");
        };
        assert_eq!(state.game_type, "cricket");
        assert_eq!(state.ruleset_version, 1);
        assert_ne!(state.random_seed, 0);
        let random_seed = state.random_seed;
        assert_eq!(state.players[0].marks.get("20"), Some(&0));
        assert_eq!(
            state.overlay["prompt"],
            serde_json::json!("Offene Cricket-Ziele")
        );

        let invalid = runtime
            .dispatch_envelope(CommandEnvelope {
                protocol_version: PROTOCOL_VERSION,
                command_id: "invalid-cricket".into(),
                runtime_instance_id: "runtime".into(),
                expected_revision: Some(1),
                command: RuntimeCommand::StartGame {
                    game_type: "cricket".into(),
                    player_ids: vec!["ada".into()],
                    options: serde_json::json!({"unknown": true}),
                },
            })
            .expect_err("unknown options must be rejected");
        assert_eq!(invalid.code, ErrorCode::InvalidCommand);
        assert_eq!(runtime.snapshot().revision, 1);
        runtime
            .dispatch(
                "runtime",
                "cricket-t20",
                Some(1),
                RuntimeAction::Dart {
                    event: DartEvent::Hit {
                        seq: 1,
                        field: 20,
                        ring: Ring::Triple,
                        multiplier: 3,
                        label: "T20".into(),
                        score: 60,
                    },
                    source: DartSource::Board,
                },
            )
            .expect("Cricket dart");
        let repository = runtime.into_repository();
        let restored = Runtime::restore("restored-runtime", repository).expect("restore Cricket");
        let Some(RuntimeGame::Registered(game)) = restored.snapshot().game.as_ref() else {
            panic!("registered game was not restored");
        };
        assert_eq!(game.state().players[0].marks.get("20"), Some(&3));
        assert_eq!(game.state().random_seed, random_seed);
        assert_eq!(restored.snapshot().revision, 2);
    }

    #[test]
    fn command_envelope_returns_stable_revision_and_instance_errors() {
        let repository = MemoryRepository::default();
        let mut runtime = Runtime::restore("runtime", repository).expect("runtime");
        let wrong_instance = runtime
            .dispatch_envelope(CommandEnvelope {
                protocol_version: PROTOCOL_VERSION,
                command_id: "wrong-instance".into(),
                runtime_instance_id: "other".into(),
                expected_revision: Some(0),
                command: RuntimeCommand::Undo,
            })
            .expect_err("instance must be rejected");
        assert_eq!(wrong_instance.code, ErrorCode::WrongRuntimeInstance);

        let stale = runtime
            .dispatch_envelope(CommandEnvelope {
                protocol_version: PROTOCOL_VERSION,
                command_id: "stale".into(),
                runtime_instance_id: "runtime".into(),
                expected_revision: Some(7),
                command: RuntimeCommand::StartGame {
                    game_type: "countup".into(),
                    player_ids: vec!["ada".into()],
                    options: serde_json::json!({"rounds": 8}),
                },
            })
            .expect_err("revision must be rejected");
        assert_eq!(stale.code, ErrorCode::StaleRevision);
    }

    #[test]
    fn next_player_finishes_the_last_partial_visit_and_undoes_session_points() {
        let repository = MemoryRepository::default();
        let mut runtime = Runtime::restore("runtime", repository).expect("runtime");
        runtime
            .dispatch(
                "runtime",
                "session",
                None,
                RuntimeAction::StartSession {
                    session_id: "session-1".into(),
                    players: session_players().into_iter().take(1).collect(),
                    teams: Vec::new(),
                },
            )
            .expect("session");
        runtime
            .dispatch(
                "runtime",
                "prepare",
                None,
                RuntimeAction::PrepareGame {
                    game_type: "countup".into(),
                    options: serde_json::json!({"rounds": 1}),
                },
            )
            .expect("prepare");
        runtime
            .dispatch(
                "runtime",
                "start",
                None,
                RuntimeAction::StartPreparedGame {
                    game_id: "game-1".into(),
                },
            )
            .expect("start");
        runtime
            .dispatch("runtime", "playing", None, RuntimeAction::MarkGamePlaying)
            .expect("playing");
        runtime
            .dispatch(
                "runtime",
                "partial-dart",
                None,
                RuntimeAction::Dart {
                    event: DartEvent::Hit {
                        seq: 1,
                        field: 20,
                        ring: Ring::Triple,
                        multiplier: 3,
                        label: "T20".into(),
                        score: 60,
                    },
                    source: DartSource::Board,
                },
            )
            .expect("partial visit");
        runtime
            .dispatch("runtime", "next-player", None, RuntimeAction::NextPlayer)
            .expect("finish skipped visit");

        assert_eq!(runtime.snapshot.session.state().screen, Screen::GameResult);
        assert_eq!(
            runtime.snapshot.session.state().standings[0].session_points,
            3
        );
        let Some(RuntimeGame::CountUp(game)) = runtime.snapshot.game.as_ref() else {
            panic!("wrong game type");
        };
        assert_eq!(game.state().status, GameStatus::Finished);

        runtime
            .dispatch("runtime", "undo-next-player", None, RuntimeAction::Undo)
            .expect("undo next player");
        assert_eq!(runtime.snapshot.session.state().screen, Screen::Playing);
        assert_eq!(
            runtime.snapshot.session.state().standings[0].session_points,
            0
        );
        let Some(RuntimeGame::CountUp(game)) = runtime.snapshot.game.as_ref() else {
            panic!("wrong game type");
        };
        assert_eq!(game.state().status, GameStatus::Running);
        assert_eq!(game.state().darts_in_turn, 1);
        assert_eq!(game.state().players[0].score, 60);
    }

    #[test]
    fn session_result_and_game_state_commit_or_rollback_together() {
        let repository = MemoryRepository::default();
        let mut runtime = Runtime::restore("runtime", repository).expect("runtime");
        runtime
            .dispatch(
                "runtime",
                "session",
                None,
                RuntimeAction::StartSession {
                    session_id: "session-1".into(),
                    players: session_players(),
                    teams: Vec::new(),
                },
            )
            .expect("session");
        runtime
            .dispatch(
                "runtime",
                "prepare",
                None,
                RuntimeAction::PrepareGame {
                    game_type: "x01".into(),
                    options: serde_json::json!({
                        "start_score": 40,
                        "out_rule": "double"
                    }),
                },
            )
            .expect("prepare");
        runtime
            .dispatch(
                "runtime",
                "start",
                None,
                RuntimeAction::StartPreparedGame {
                    game_id: "game-1".into(),
                },
            )
            .expect("start");
        runtime
            .dispatch("runtime", "playing", None, RuntimeAction::MarkGamePlaying)
            .expect("playing");
        let checkout = DartEvent::Hit {
            seq: 1,
            field: 20,
            ring: Ring::Double,
            multiplier: 2,
            label: "D20".into(),
            score: 40,
        };
        runtime
            .dispatch(
                "runtime",
                "checkout",
                None,
                RuntimeAction::Dart {
                    event: checkout.clone(),
                    source: DartSource::Board,
                },
            )
            .expect("checkout");
        assert_eq!(runtime.snapshot.session.state().screen, Screen::GameResult);
        assert_eq!(
            runtime.snapshot.session.state().standings[0].session_points,
            3
        );

        runtime
            .dispatch("runtime", "undo", None, RuntimeAction::Undo)
            .expect("undo checkout");
        assert_eq!(runtime.snapshot.session.state().screen, Screen::Playing);
        assert_eq!(
            runtime.snapshot.session.state().standings[0].session_points,
            0
        );

        runtime.repository.fail_next_commit();
        let error = runtime
            .dispatch(
                "runtime",
                "failed-checkout",
                None,
                RuntimeAction::Dart {
                    event: checkout,
                    source: DartSource::Board,
                },
            )
            .expect_err("commit must fail");
        assert!(matches!(error, RuntimeError::Persistence(_)));
        assert_eq!(runtime.snapshot.session.state().screen, Screen::Playing);
        assert_eq!(
            runtime.snapshot.session.state().standings[0].session_points,
            0
        );
        let Some(RuntimeGame::X01(game)) = &runtime.snapshot.game else {
            panic!("wrong game type");
        };
        assert_eq!(game.state().players[0].score, 40);
    }

    #[test]
    fn prepared_registry_game_preserves_session_profiles() {
        let game = game_from_options("eight_ball", session_players(), &serde_json::json!({}), 42)
            .expect("registered game");
        let RuntimeGame::Registered(game) = game else {
            panic!("wrong game type");
        };

        assert_eq!(game.state().players[0].avatar, "nova");
        assert_eq!(game.state().players[0].color, "#ff00aa");
        assert_eq!(game.state().players[1].avatar, "comet");
        assert_eq!(game.state().players[1].color, "#28e7ff");
    }

    #[test]
    fn shared_settings_are_validated_persisted_and_published() {
        let repository = MemoryRepository::default();
        let mut runtime = Runtime::restore("runtime", repository).expect("runtime");
        let invalid = runtime
            .dispatch_envelope(CommandEnvelope {
                protocol_version: PROTOCOL_VERSION,
                command_id: "invalid-geometry".into(),
                runtime_instance_id: "runtime".into(),
                expected_revision: Some(0),
                command: RuntimeCommand::ReportProjectorGeometry {
                    geometry: ProjectorGeometry {
                        width: 100,
                        height: 100,
                    },
                },
            })
            .expect_err("invalid geometry");
        assert_eq!(invalid.code, ErrorCode::InvalidCommand);
        assert_eq!(runtime.snapshot().revision, 0);
        runtime
            .dispatch_envelope(CommandEnvelope {
                protocol_version: PROTOCOL_VERSION,
                command_id: "geometry".into(),
                runtime_instance_id: "runtime".into(),
                expected_revision: Some(0),
                command: RuntimeCommand::ReportProjectorGeometry {
                    geometry: ProjectorGeometry {
                        width: 2_000,
                        height: 1_000,
                    },
                },
            })
            .expect("geometry");
        runtime
            .dispatch("runtime", "reset", None, RuntimeAction::ResetCalibration)
            .expect("reset calibration");
        let corners = runtime.snapshot().settings.calibration.corners;
        assert!((corners[0].x - 0.275).abs() < f64::EPSILON);
        assert!((corners[0].y - 0.05).abs() < f64::EPSILON);
        assert!((corners[2].x - 0.725).abs() < f64::EPSILON);
        assert!((corners[2].y - 0.95).abs() < f64::EPSILON);

        runtime
            .dispatch(
                "runtime",
                "theme",
                None,
                RuntimeAction::UpdateArtTheme {
                    theme: ArtTheme::Neon,
                },
            )
            .expect("theme");
        runtime
            .dispatch(
                "runtime",
                "sound",
                None,
                RuntimeAction::UpdateSoundSettings {
                    enabled: true,
                    output: SoundOutput::Both,
                },
            )
            .expect("sound");

        let repository = runtime.into_repository();
        let restored = Runtime::restore("restored", repository).expect("restore");
        assert_eq!(restored.snapshot().settings.art_theme, ArtTheme::Neon);
        assert_eq!(restored.snapshot().settings.sound.output, SoundOutput::Both);
        assert_eq!(
            restored.snapshot().settings.sound.status,
            SoundStatus::Starting
        );
        let public = restored.public_snapshot();
        assert_eq!(public.settings, restored.snapshot().settings);
    }

    #[test]
    fn sound_effects_are_committed_deduplicated_and_acknowledged() {
        let repository = MemoryRepository::default();
        let mut runtime = Runtime::restore("runtime", repository).expect("runtime");
        runtime
            .dispatch(
                "runtime",
                "sound-on",
                None,
                RuntimeAction::UpdateSoundSettings {
                    enabled: true,
                    output: SoundOutput::Both,
                },
            )
            .expect("sound");
        runtime
            .dispatch(
                "runtime",
                "start",
                None,
                RuntimeAction::StartCountUp {
                    players: players(),
                    rounds: 5,
                },
            )
            .expect("game");
        let action = RuntimeAction::Dart {
            event: DartEvent::Hit {
                seq: 7,
                field: 20,
                ring: Ring::Triple,
                multiplier: 3,
                label: "T20".into(),
                score: 60,
            },
            source: DartSource::Board,
        };
        runtime
            .dispatch("runtime", "dart-7", None, action.clone())
            .expect("dart");
        let effects = runtime.public_snapshot().effects;
        assert_eq!(effects.len(), 3);
        assert!(effects.iter().all(|effect| effect.revision == 3));
        assert!(
            effects
                .iter()
                .any(|effect| effect.delivery == EffectDelivery::Discardable)
        );
        let bootstrap_effects = runtime.bootstrap_snapshot().effects;
        assert_eq!(bootstrap_effects.len(), 2);
        assert!(
            bootstrap_effects
                .iter()
                .all(|effect| effect.delivery != EffectDelivery::Discardable)
        );
        let duplicate = runtime
            .dispatch("runtime", "dart-7", None, action)
            .expect("duplicate");
        assert!(duplicate.duplicate);
        assert_eq!(runtime.public_snapshot().effects.len(), 3);

        let controller = effects
            .iter()
            .find(|effect| effect.target == EffectTarget::Controller)
            .expect("controller effect");
        assert!(
            runtime
                .acknowledge_effect(&controller.effect_id, EffectTarget::Controller)
                .expect("ack")
        );
        assert_eq!(runtime.public_snapshot().effects.len(), 2);
        assert!(
            !runtime
                .acknowledge_effect(&controller.effect_id, EffectTarget::Controller)
                .expect("duplicate ack")
        );

        runtime
            .dispatch(
                "runtime",
                "theme",
                None,
                RuntimeAction::UpdateArtTheme {
                    theme: ArtTheme::Neon,
                },
            )
            .expect("newer revision");
        assert!(runtime.public_snapshot().effects.is_empty());
    }

    #[test]
    fn correction_lock_pauses_physical_but_not_manual_darts() {
        let repository = MemoryRepository::default();
        let mut runtime = Runtime::restore("runtime", repository).expect("runtime");
        runtime
            .dispatch(
                "runtime",
                "start",
                None,
                RuntimeAction::StartCountUp {
                    players: vec![("ada".into(), "Ada".into())],
                    rounds: 5,
                },
            )
            .expect("start game");
        runtime
            .dispatch(
                "runtime",
                "lock",
                None,
                RuntimeAction::SetCorrectionLock { active: true },
            )
            .expect("correction lock");
        let dart = DartEvent::Hit {
            seq: 1,
            field: 20,
            ring: Ring::Triple,
            multiplier: 3,
            label: "T20".into(),
            score: 60,
        };
        assert_eq!(
            runtime
                .dispatch(
                    "runtime",
                    "blocked-board-dart",
                    None,
                    RuntimeAction::Dart {
                        event: dart.clone(),
                        source: DartSource::Board,
                    },
                )
                .expect_err("board must pause"),
            RuntimeError::CorrectionLocked
        );
        runtime
            .dispatch(
                "runtime",
                "manual-dart",
                None,
                RuntimeAction::Dart {
                    event: dart,
                    source: DartSource::ManualCorrection,
                },
            )
            .expect("manual correction remains available");
        let repository = runtime.into_repository();
        let restored = Runtime::restore("restored", repository).expect("restore");
        assert!(!restored.snapshot().settings.correction_lock);
    }
}
