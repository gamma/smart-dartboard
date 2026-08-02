//! Shared, privacy-preserving diagnostics for hosted and native runtimes.

use serde::Serialize;
use serde_json::{Map, Value, json};
use std::{
    collections::{BTreeMap, VecDeque, hash_map::DefaultHasher},
    fs::{self, OpenOptions},
    hash::{Hash, Hasher},
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

const LOG_FILE: &str = "diagnostics.jsonl";
const EXPORT_SCHEMA_VERSION: u16 = 1;
const DEFAULT_MAX_BYTES: u64 = 1_048_576;
const DEFAULT_FILES: usize = 5;
const DEFAULT_EXPORT_RECORDS: usize = 2_000;

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticVersions {
    pub app: String,
    pub contract: u16,
    pub schema: u32,
    pub rulesets: BTreeMap<String, u16>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticEnvironment {
    pub platform: String,
    pub os: String,
    pub os_version: String,
    pub adapter: String,
    pub adapter_version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticContext {
    pub versions: DiagnosticVersions,
    pub environment: DiagnosticEnvironment,
}

impl DiagnosticEnvironment {
    #[must_use]
    pub fn current(adapter: impl Into<String>, adapter_version: impl Into<String>) -> Self {
        Self {
            platform: std::env::consts::FAMILY.into(),
            os: std::env::consts::OS.into(),
            os_version: os_version(),
            adapter: adapter.into(),
            adapter_version: adapter_version.into(),
        }
    }
}

fn os_version() -> String {
    std::env::var("SDB_OS_VERSION")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::process::Command::new("uname")
                .arg("-r")
                .output()
                .ok()
                .filter(|output| output.status.success())
                .and_then(|output| String::from_utf8(output.stdout).ok())
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_else(|| "unknown".into())
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticLevel {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DiagnosticScope<'a> {
    pub runtime_instance_id: Option<&'a str>,
    pub revision: Option<u64>,
    pub session_id: Option<&'a str>,
    pub game_id: Option<&'a str>,
    pub event_id: Option<&'a str>,
    pub board_state: Option<&'a str>,
    pub adapter_error_code: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticExport {
    pub export_schema_version: u16,
    pub created_at_ms: u64,
    pub health: Value,
    pub versions: DiagnosticVersions,
    pub environment: DiagnosticEnvironment,
    pub configuration: Value,
    pub logs: Vec<Value>,
    pub database_included: bool,
}

#[derive(Debug)]
enum Sink {
    Disk {
        directory: PathBuf,
        max_bytes: u64,
        files: usize,
    },
    Memory {
        records: VecDeque<Value>,
    },
}

#[derive(Debug)]
struct Inner {
    context: DiagnosticContext,
    sink: Sink,
}

#[derive(Debug, Clone)]
pub struct DiagnosticLogger(Arc<Mutex<Inner>>);

impl DiagnosticLogger {
    /// Opens a rotating JSONL logger with production defaults.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when the log directory cannot be created.
    pub fn open(directory: impl AsRef<Path>, context: DiagnosticContext) -> io::Result<Self> {
        Self::open_with_limits(directory, context, DEFAULT_MAX_BYTES, DEFAULT_FILES)
    }

    /// Opens a rotating JSONL logger with explicit size and retention limits.
    ///
    /// # Errors
    ///
    /// Returns an I/O error for invalid limits or inaccessible storage.
    pub fn open_with_limits(
        directory: impl AsRef<Path>,
        context: DiagnosticContext,
        max_bytes: u64,
        files: usize,
    ) -> io::Result<Self> {
        let directory = directory.as_ref().to_path_buf();
        fs::create_dir_all(&directory)?;
        if max_bytes == 0 || files == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "diagnostic rotation limits must be positive",
            ));
        }
        Ok(Self(Arc::new(Mutex::new(Inner {
            context,
            sink: Sink::Disk {
                directory,
                max_bytes,
                files,
            },
        }))))
    }

    #[must_use]
    pub fn memory(context: DiagnosticContext) -> Self {
        Self(Arc::new(Mutex::new(Inner {
            context,
            sink: Sink::Memory {
                records: VecDeque::new(),
            },
        })))
    }

    /// Appends one structured and redacted diagnostic record.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when the logger lock or persistent sink fails.
    pub fn record(
        &self,
        level: DiagnosticLevel,
        component: &str,
        event: &str,
        scope: DiagnosticScope<'_>,
        fields: Value,
    ) -> io::Result<()> {
        let mut inner = self
            .0
            .lock()
            .map_err(|_| io::Error::other("diagnostic logger lock poisoned"))?;
        let context = inner.context.clone();
        let record = json!({
            "timestamp_ms": now_ms(),
            "level": level,
            "component": safe_label(component),
            "event": safe_label(event),
            "versions": context.versions,
            "environment": context.environment,
            "runtime_instance_id": scope.runtime_instance_id.map(anonymize_id),
            "revision": scope.revision,
            "session_id": scope.session_id.map(anonymize_id),
            "game_id": scope.game_id.map(anonymize_id),
            "event_id": scope.event_id.map(anonymize_id),
            "board_state": scope.board_state.map(safe_label),
            "adapter_error_code": scope.adapter_error_code.map(safe_label),
            "fields": redact(fields),
        });
        match &mut inner.sink {
            Sink::Memory { records } => {
                records.push_back(record);
                while records.len() > DEFAULT_EXPORT_RECORDS {
                    records.pop_front();
                }
                Ok(())
            }
            Sink::Disk {
                directory,
                max_bytes,
                files,
            } => write_disk(directory, *max_bytes, *files, &record),
        }
    }

    /// Builds a redacted support bundle without including the database.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when retained logs cannot be read.
    pub fn export(&self, health: Value, configuration: Value) -> io::Result<DiagnosticExport> {
        let inner = self
            .0
            .lock()
            .map_err(|_| io::Error::other("diagnostic logger lock poisoned"))?;
        let logs = match &inner.sink {
            Sink::Memory { records } => records.iter().cloned().collect(),
            Sink::Disk {
                directory, files, ..
            } => read_disk(directory, *files)?,
        };
        Ok(DiagnosticExport {
            export_schema_version: EXPORT_SCHEMA_VERSION,
            created_at_ms: now_ms(),
            health: redact(health),
            versions: inner.context.versions.clone(),
            environment: inner.context.environment.clone(),
            configuration: redact(configuration),
            logs,
            database_included: false,
        })
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn safe_label(value: &str) -> String {
    value
        .chars()
        .filter(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
        })
        .take(96)
        .collect()
}

fn anonymize_id(value: &str) -> String {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    format!("anon-{:016x}", hasher.finish())
}

fn is_sensitive_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    [
        "name",
        "token",
        "secret",
        "password",
        "authorization",
        "certificate",
        "fingerprint",
        "raw",
        "packet",
        "payload",
        "address",
        "device_id",
        "connection_id",
        "host_id",
        "detail",
    ]
    .iter()
    .any(|needle| key.contains(needle))
}

fn allows_string_value(key: &str) -> bool {
    matches!(
        key,
        "status"
            | "runtime"
            | "database"
            | "board"
            | "board_failure_code"
            | "companion"
            | "app_role"
            | "projector_output"
            | "command_type"
            | "target"
    )
}

fn redact(value: Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .into_iter()
                .map(|(key, value)| {
                    if is_sensitive_key(&key) {
                        (key, Value::String("[redacted]".into()))
                    } else if let Value::String(value) = value {
                        if allows_string_value(&key) {
                            (key, Value::String(safe_label(&value)))
                        } else {
                            (key, Value::String("[redacted]".into()))
                        }
                    } else {
                        (key, redact(value))
                    }
                })
                .collect::<Map<_, _>>(),
        ),
        Value::Array(values) => Value::Array(values.into_iter().map(redact).collect()),
        Value::String(_) => Value::String("[redacted]".into()),
        other => other,
    }
}

fn log_path(directory: &Path, index: usize) -> PathBuf {
    if index == 0 {
        directory.join(LOG_FILE)
    } else {
        directory.join(format!("{LOG_FILE}.{index}"))
    }
}

fn write_disk(directory: &Path, max_bytes: u64, files: usize, record: &Value) -> io::Result<()> {
    let mut line = serde_json::to_vec(record).map_err(io::Error::other)?;
    line.push(b'\n');
    let current = log_path(directory, 0);
    let current_size = current.metadata().map_or(0, |metadata| metadata.len());
    if current_size > 0 && current_size.saturating_add(line.len() as u64) > max_bytes {
        rotate(directory, files)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(current)?;
    file.write_all(&line)?;
    file.flush()
}

fn rotate(directory: &Path, files: usize) -> io::Result<()> {
    let oldest = log_path(directory, files.saturating_sub(1));
    if files > 1 && oldest.exists() {
        fs::remove_file(oldest)?;
    }
    for index in (1..files).rev() {
        let source = log_path(directory, index - 1);
        if source.exists() {
            fs::rename(source, log_path(directory, index))?;
        }
    }
    if files == 1 {
        let current = log_path(directory, 0);
        if current.exists() {
            fs::remove_file(current)?;
        }
    }
    Ok(())
}

fn read_disk(directory: &Path, files: usize) -> io::Result<Vec<Value>> {
    let mut records = VecDeque::new();
    for index in (0..files).rev() {
        let path = log_path(directory, index);
        if !path.exists() {
            continue;
        }
        for line in BufReader::new(fs::File::open(path)?).lines() {
            if let Ok(value) = serde_json::from_str::<Value>(&line?) {
                records.push_back(value);
                while records.len() > DEFAULT_EXPORT_RECORDS {
                    records.pop_front();
                }
            }
        }
    }
    Ok(records.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> DiagnosticContext {
        DiagnosticContext {
            versions: DiagnosticVersions {
                app: "0.1.0".into(),
                contract: 2,
                schema: 6,
                rulesets: BTreeMap::from([("x01".into(), 1)]),
            },
            environment: DiagnosticEnvironment::current("test", "1"),
        }
    }

    #[test]
    fn redacts_secrets_names_raw_packets_and_identifiers() {
        let logger = DiagnosticLogger::memory(context());
        logger
            .record(
                DiagnosticLevel::Error,
                "board adapter",
                "packet rejected",
                DiagnosticScope {
                    runtime_instance_id: Some("runtime-private"),
                    session_id: Some("session-private"),
                    adapter_error_code: Some("bad packet"),
                    ..DiagnosticScope::default()
                },
                json!({
                    "player_name": "Gerry",
                    "board_token": "secret",
                    "raw_hex": "deadbeef",
                    "safe_count": 7,
                    "unexpected_text": "must disappear",
                }),
            )
            .expect("record");
        let export = logger
            .export(json!({"status":"ok"}), json!({"token":"nope"}))
            .expect("export");
        let serialized = serde_json::to_string(&export).expect("serialize");
        assert!(!serialized.contains("Gerry"));
        assert!(!serialized.contains("secret"));
        assert!(!serialized.contains("deadbeef"));
        assert!(!serialized.contains("runtime-private"));
        assert!(serialized.contains("anon-"));
        assert!(!serialized.contains("must disappear"));
        assert!(serialized.contains("safe_count"));
        assert!(!export.database_included);
    }

    #[test]
    fn rotates_files_and_exports_records_in_order() {
        let directory = std::env::temp_dir().join(format!("sdb-diagnostics-{}", now_ms()));
        let logger =
            DiagnosticLogger::open_with_limits(&directory, context(), 700, 3).expect("logger");
        for index in 0..8 {
            logger
                .record(
                    DiagnosticLevel::Info,
                    "runtime",
                    "revision_committed",
                    DiagnosticScope {
                        revision: Some(index),
                        ..DiagnosticScope::default()
                    },
                    json!({}),
                )
                .expect("record");
        }
        assert!(log_path(&directory, 1).exists());
        assert!(!log_path(&directory, 3).exists());
        let export = logger.export(json!({}), json!({})).expect("export");
        let revisions: Vec<_> = export
            .logs
            .iter()
            .filter_map(|value| value["revision"].as_u64())
            .collect();
        assert!(revisions.windows(2).all(|window| window[0] < window[1]));
        fs::remove_dir_all(directory).expect("cleanup");
    }
}
