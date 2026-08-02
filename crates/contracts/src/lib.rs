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
