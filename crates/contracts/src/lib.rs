//! Versioned messages shared by the headless server, native shells and web UI.
//!
//! This crate deliberately contains no transport, storage or operating-system
//! dependencies. Every supported host serializes these same contracts.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROTOCOL_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageKind {
    State,
    Command,
    Event,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Envelope<T> {
    pub protocol_version: u16,
    pub runtime_instance_id: String,
    pub message_id: String,
    pub revision: u64,
    pub kind: MessageKind,
    pub payload: T,
}

impl<T> Envelope<T> {
    #[must_use]
    pub fn new(
        runtime_instance_id: impl Into<String>,
        message_id: impl Into<String>,
        revision: u64,
        kind: MessageKind,
        payload: T,
    ) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            runtime_instance_id: runtime_instance_id.into(),
            message_id: message_id.into(),
            revision,
            kind,
            payload,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerRef {
    pub id: String,
    pub name: String,
    pub avatar: String,
    pub color: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StarterSelection {
    #[default]
    Rotation,
    Manual,
    Random,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ring {
    SingleInner,
    SingleOuter,
    Triple,
    Double,
    SingleBull,
    DoubleBull,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DartSource {
    #[default]
    Board,
    ProjectorTest,
    ManualCorrection,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CalibrationPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalibrationSettings {
    pub corners: [CalibrationPoint; 4],
    pub scale: f64,
    pub offset_x: f64,
    pub offset_y: f64,
}

impl Default for CalibrationSettings {
    fn default() -> Self {
        Self {
            corners: [
                CalibrationPoint { x: 0.247, y: 0.05 },
                CalibrationPoint { x: 0.753, y: 0.05 },
                CalibrationPoint { x: 0.753, y: 0.95 },
                CalibrationPoint { x: 0.247, y: 0.95 },
            ],
            scale: 1.0,
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectorGeometry {
    pub width: u32,
    pub height: u32,
}

impl Default for ProjectorGeometry {
    fn default() -> Self {
        Self {
            width: 1_600,
            height: 900,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoundOutput {
    Controller,
    #[default]
    Projector,
    Both,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoundStatus {
    #[default]
    Disabled,
    Starting,
    Ready,
    Blocked,
    Unavailable,
}

/// Product surface that must execute a committed platform effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectTarget {
    Controller,
    Projector,
}

/// Recovery policy for an effect that has not yet been acknowledged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectDelivery {
    /// Keep retrying across revisions until a platform target acknowledges it.
    Durable,
    /// Retry after a crash only while the producing revision is still current.
    Recoverable,
    /// Deliver only to currently connected targets and never reconstruct it.
    Discardable,
}

/// Declarative effect payload. Platform hosts decide how the cue is rendered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PlatformEffectKind {
    Sound {
        cue: String,
        event: Option<DartEvent>,
    },
    Visual {
        cue: String,
        event: DartEvent,
    },
}

/// An effect committed atomically with the revision that produced it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformEffect {
    pub effect_id: String,
    pub revision: u64,
    pub target: EffectTarget,
    pub delivery: EffectDelivery,
    pub kind: PlatformEffectKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoundSettings {
    pub enabled: bool,
    pub output: SoundOutput,
    pub status: SoundStatus,
}

impl Default for SoundSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            output: SoundOutput::Projector,
            status: SoundStatus::Disabled,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtTheme {
    #[default]
    Cartoon,
    Neon,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UiLanguage {
    #[default]
    De,
    En,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplayOverride {
    Players,
    Calibration,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RuntimeSettings {
    pub calibration: CalibrationSettings,
    pub projector_geometry: ProjectorGeometry,
    pub sound: SoundSettings,
    pub art_theme: ArtTheme,
    pub ui_language: UiLanguage,
    pub correction_lock: bool,
    pub sound_test_id: Option<String>,
    pub display_override: Option<DisplayOverride>,
}

impl Default for RuntimeSettings {
    fn default() -> Self {
        Self {
            calibration: CalibrationSettings::default(),
            projector_geometry: ProjectorGeometry::default(),
            sound: SoundSettings::default(),
            art_theme: ArtTheme::Cartoon,
            ui_language: UiLanguage::De,
            correction_lock: false,
            sound_test_id: None,
            display_override: None,
        }
    }
}

impl Ring {
    #[must_use]
    pub const fn multiplier(self) -> u8 {
        match self {
            Self::SingleInner | Self::SingleOuter | Self::SingleBull => 1,
            Self::Double | Self::DoubleBull => 2,
            Self::Triple => 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DartEvent {
    Hit {
        seq: u64,
        field: u8,
        ring: Ring,
        multiplier: u8,
        label: String,
        score: u16,
    },
    Miss {
        seq: u64,
        label: String,
        score: u16,
    },
}

impl DartEvent {
    #[must_use]
    pub const fn seq(&self) -> u64 {
        match self {
            Self::Hit { seq, .. } | Self::Miss { seq, .. } => *seq,
        }
    }

    #[must_use]
    pub const fn score(&self) -> u16 {
        match self {
            Self::Hit { score, .. } | Self::Miss { score, .. } => *score,
        }
    }

    #[must_use]
    pub fn label(&self) -> &str {
        match self {
            Self::Hit { label, .. } | Self::Miss { label, .. } => label,
        }
    }

    #[must_use]
    pub const fn multiplier(&self) -> u8 {
        match self {
            Self::Hit { multiplier, .. } => *multiplier,
            Self::Miss { .. } => 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuntimeCommand {
    CreatePlayer {
        player: PlayerRef,
    },
    StartSession {
        session_id: String,
        players: Vec<PlayerRef>,
    },
    PrepareGame {
        game_type: String,
        options: Value,
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
    IngestDart {
        event: DartEvent,
        #[serde(default)]
        source: DartSource,
    },
    CorrectDart {
        action_id: u64,
        replacement: DartEvent,
        #[serde(default = "default_correction_source")]
        source: DartSource,
    },
    DeleteDart {
        action_id: u64,
    },
    StartGame {
        game_type: String,
        player_ids: Vec<String>,
        options: Value,
    },
    GameAction {
        action: String,
        payload: Value,
    },
    ContinueTurn,
    NextPlayer,
    Undo,
    AbortGame,
}

const fn default_correction_source() -> DartSource {
    DartSource::ManualCorrection
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandEnvelope {
    pub protocol_version: u16,
    pub command_id: String,
    pub runtime_instance_id: String,
    pub expected_revision: Option<u64>,
    pub command: RuntimeCommand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    IncompatibleProtocol,
    WrongRuntimeInstance,
    StaleRevision,
    InvalidCommand,
    Forbidden,
    PersistenceFailed,
    BoardUnavailable,
    NotFound,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_serialization_is_stable() {
        let envelope = Envelope::new(
            "runtime-1",
            "message-1",
            7,
            MessageKind::Event,
            DartEvent::Hit {
                seq: 3,
                field: 20,
                ring: Ring::Triple,
                multiplier: 3,
                label: "T20".into(),
                score: 60,
            },
        );
        let value = serde_json::to_value(envelope).expect("serialize envelope");
        assert_eq!(value["protocol_version"], PROTOCOL_VERSION);
        assert_eq!(value["payload"]["type"], "hit");
        assert_eq!(value["payload"]["ring"], "triple");
    }
}
