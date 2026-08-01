use axum::{
    Json, Router,
    extract::{Path, State, WebSocketUpgrade, ws::Message},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use sdb_contracts::{
    CommandEnvelope, ContractError, Envelope, ErrorCode, MessageKind, PROTOCOL_VERSION,
};
use sdb_runtime::{CommandResult, Runtime, RuntimeSnapshot};
use sdb_storage::SqliteRepository;
use serde::{Deserialize, Serialize};
use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast;
use tower_http::set_header::SetResponseHeaderLayer;
use uuid::Uuid;

type SharedRuntime = Arc<Mutex<Runtime<SqliteRepository>>>;
type StateMessage = Envelope<RuntimeSnapshot>;

#[derive(Clone)]
struct AppState {
    runtime: SharedRuntime,
    states: broadcast::Sender<StateMessage>,
    board_status: &'static str,
}

#[derive(Serialize)]
struct Health {
    status: &'static str,
    runtime: &'static str,
    database: &'static str,
    board: &'static str,
    protocol_version: u16,
    schema_version: u32,
    revision: u64,
}

#[derive(Serialize)]
struct ServiceInfo {
    service: &'static str,
    api: &'static str,
    production_replacement: bool,
}

#[derive(Debug, Deserialize)]
struct HistoryQuery {
    limit: Option<usize>,
}

#[tokio::main]
async fn main() {
    let data_dir = PathBuf::from(env::var("SDB_DATA_DIR").unwrap_or_else(|_| "data".into()));
    std::fs::create_dir_all(&data_dir).expect("create data directory");
    let repository =
        SqliteRepository::open(data_dir.join("runtime.sqlite")).expect("open runtime database");
    let runtime = Runtime::restore(Uuid::new_v4().to_string(), repository)
        .expect("restore committed runtime");
    let ble_enabled = env_flag("SDB_ENABLE_BLE", true);
    let state = AppState::new(
        runtime,
        if ble_enabled {
            "unavailable"
        } else {
            "disabled"
        },
    );
    let app = router(state);
    let port = env::var("SDB_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(8000);
    let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("bind server socket");
    println!("Smart Dartboard runtime v2 listening on http://{address}");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("serve HTTP");
}

impl AppState {
    fn new(runtime: Runtime<SqliteRepository>, board_status: &'static str) -> Self {
        let (states, _) = broadcast::channel(64);
        Self {
            runtime: Arc::new(Mutex::new(runtime)),
            states,
            board_status,
        }
    }

    fn snapshot(&self, message_id: impl Into<String>) -> Result<StateMessage, ContractError> {
        let runtime = self
            .runtime
            .lock()
            .map_err(|_| internal_error("runtime lock poisoned"))?;
        Ok(Envelope::new(
            runtime.instance_id(),
            message_id,
            runtime.snapshot().revision,
            MessageKind::State,
            runtime.snapshot().clone(),
        ))
    }
}

fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(service_info))
        .route("/api/v2/health", get(health))
        .route("/api/v2/runtime/bootstrap", get(bootstrap))
        .route("/api/v2/runtime/snapshot", get(snapshot))
        .route("/api/v2/runtime/commands", post(command))
        .route("/api/v2/runtime/events", get(websocket))
        .route("/api/v2/players", get(players))
        .route("/api/v2/history/sessions", get(session_history))
        .route("/api/v2/history/sessions/{session_id}", get(session_detail))
        .route("/api/v2/history/games/{game_id}", get(game_detail))
        .route(
            "/api/v2/history/games/{game_id}/replay",
            get(game_replay),
        )
        .route("/api/v2/statistics/players", get(player_statistics))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::REFERRER_POLICY,
            HeaderValue::from_static("no-referrer"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(
                "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; script-src 'self'; connect-src 'self' ws: wss:; frame-ancestors 'none'; base-uri 'none'; form-action 'self'",
            ),
        ))
        .with_state(state)
}

async fn service_info() -> Json<ServiceInfo> {
    Json(ServiceInfo {
        service: "Smart Dartboard Rust Runtime",
        api: "v2-preview",
        production_replacement: false,
    })
}

async fn health(State(state): State<AppState>) -> Result<Json<Health>, ApiError> {
    let runtime = state
        .runtime
        .lock()
        .map_err(|_| internal_error("runtime lock poisoned"))?;
    let board_ready = matches!(state.board_status, "ready" | "disabled");
    let schema_version = runtime
        .repository()
        .schema_version()
        .map_err(|_| internal_error("database schema query failed"))?;
    Ok(Json(Health {
        status: if board_ready { "ok" } else { "degraded" },
        runtime: "ok",
        database: "ok",
        board: state.board_status,
        protocol_version: PROTOCOL_VERSION,
        schema_version,
        revision: runtime.snapshot().revision,
    }))
}

async fn bootstrap(State(state): State<AppState>) -> Result<Json<StateMessage>, ApiError> {
    Ok(Json(state.snapshot(Uuid::new_v4().to_string())?))
}

async fn snapshot(State(state): State<AppState>) -> Result<Json<StateMessage>, ApiError> {
    Ok(Json(state.snapshot(Uuid::new_v4().to_string())?))
}

async fn players(
    State(state): State<AppState>,
) -> Result<Json<Vec<sdb_storage::PlayerProfile>>, ApiError> {
    let runtime = state
        .runtime
        .lock()
        .map_err(|_| internal_error("runtime lock poisoned"))?;
    let players = runtime
        .repository()
        .players()
        .map_err(|_| internal_error("player profile query failed"))?;
    Ok(Json(players))
}

async fn session_history(
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<HistoryQuery>,
) -> Result<Json<Vec<sdb_storage::SessionHistory>>, ApiError> {
    let runtime = state
        .runtime
        .lock()
        .map_err(|_| internal_error("runtime lock poisoned"))?;
    let sessions = runtime
        .repository()
        .sessions(query.limit.unwrap_or(50))
        .map_err(|_| internal_error("session history query failed"))?;
    Ok(Json(sessions))
}

async fn player_statistics(
    State(state): State<AppState>,
) -> Result<Json<Vec<sdb_storage::PlayerStatistics>>, ApiError> {
    let runtime = state
        .runtime
        .lock()
        .map_err(|_| internal_error("runtime lock poisoned"))?;
    let statistics = runtime
        .repository()
        .player_statistics()
        .map_err(|_| internal_error("player statistics query failed"))?;
    Ok(Json(statistics))
}

async fn session_detail(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<sdb_storage::SessionDetail>, ApiError> {
    let runtime = state
        .runtime
        .lock()
        .map_err(|_| internal_error("runtime lock poisoned"))?;
    let detail = runtime
        .repository()
        .session_detail(&session_id)
        .map_err(|_| internal_error("session detail query failed"))?
        .ok_or_else(|| not_found("session not found"))?;
    Ok(Json(detail))
}

async fn game_detail(
    State(state): State<AppState>,
    Path(game_id): Path<String>,
) -> Result<Json<sdb_storage::GameDetail>, ApiError> {
    let runtime = state
        .runtime
        .lock()
        .map_err(|_| internal_error("runtime lock poisoned"))?;
    let detail = runtime
        .repository()
        .game_detail(&game_id)
        .map_err(|_| internal_error("game detail query failed"))?
        .ok_or_else(|| not_found("game not found"))?;
    Ok(Json(detail))
}

async fn game_replay(
    State(state): State<AppState>,
    Path(game_id): Path<String>,
) -> Result<Json<sdb_storage::GameReplay>, ApiError> {
    let runtime = state
        .runtime
        .lock()
        .map_err(|_| internal_error("runtime lock poisoned"))?;
    let replay = runtime
        .repository()
        .game_replay(&game_id)
        .map_err(|_| internal_error("game replay query failed"))?
        .ok_or_else(|| not_found("game not found"))?;
    Ok(Json(replay))
}

async fn command(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(envelope): Json<CommandEnvelope>,
) -> Result<Json<CommandResult>, ApiError> {
    validate_same_origin(&headers)?;
    let command_id = envelope.command_id.clone();
    let result = {
        let mut runtime = state
            .runtime
            .lock()
            .map_err(|_| internal_error("runtime lock poisoned"))?;
        runtime.dispatch_envelope(envelope)?
    };
    let message = state.snapshot(format!("{command_id}:state"))?;
    let _ = state.states.send(message);
    Ok(Json(result))
}

async fn websocket(
    State(state): State<AppState>,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> Result<Response, ApiError> {
    validate_websocket_origin(&headers)?;
    let initial = state.snapshot(Uuid::new_v4().to_string())?;
    let receiver = state.states.subscribe();
    Ok(upgrade.on_upgrade(move |socket| stream_states(socket, initial, receiver)))
}

async fn stream_states(
    mut socket: axum::extract::ws::WebSocket,
    initial: StateMessage,
    mut receiver: broadcast::Receiver<StateMessage>,
) {
    if send_state(&mut socket, &initial).await.is_err() {
        return;
    }
    while let Ok(message) = receiver.recv().await {
        if send_state(&mut socket, &message).await.is_err() {
            break;
        }
    }
}

async fn send_state(
    socket: &mut axum::extract::ws::WebSocket,
    state: &StateMessage,
) -> Result<(), axum::Error> {
    let json = serde_json::to_string(state).map_err(axum::Error::new)?;
    socket.send(Message::Text(json.into())).await
}

fn validate_websocket_origin(headers: &HeaderMap) -> Result<(), ContractError> {
    validate_same_origin(headers)
}

fn validate_same_origin(headers: &HeaderMap) -> Result<(), ContractError> {
    let Some(origin) = headers.get(header::ORIGIN) else {
        return Ok(());
    };
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| forbidden("missing host header"))?;
    let origin = origin
        .to_str()
        .map_err(|_| forbidden("invalid origin header"))?;
    let origin_host = origin
        .strip_prefix("http://")
        .or_else(|| origin.strip_prefix("https://"))
        .ok_or_else(|| forbidden("invalid WebSocket origin"))?;
    if origin_host != host {
        return Err(forbidden("cross-origin WebSocket denied"));
    }
    Ok(())
}

fn env_flag(name: &str, default: bool) -> bool {
    env::var(name).map_or(default, |value| {
        !matches!(value.to_ascii_lowercase().as_str(), "0" | "false" | "no")
    })
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

fn forbidden(message: &str) -> ContractError {
    ContractError {
        code: ErrorCode::Forbidden,
        message: message.into(),
        details: None,
    }
}

fn internal_error(message: &str) -> ContractError {
    ContractError {
        code: ErrorCode::Internal,
        message: message.into(),
        details: None,
    }
}

fn not_found(message: &str) -> ContractError {
    ContractError {
        code: ErrorCode::NotFound,
        message: message.into(),
        details: None,
    }
}

struct ApiError(ContractError);

impl From<ContractError> for ApiError {
    fn from(error: ContractError) -> Self {
        Self(error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.0.code {
            ErrorCode::IncompatibleProtocol => StatusCode::UPGRADE_REQUIRED,
            ErrorCode::WrongRuntimeInstance | ErrorCode::StaleRevision => StatusCode::CONFLICT,
            ErrorCode::InvalidCommand | ErrorCode::BoardUnavailable => StatusCode::BAD_REQUEST,
            ErrorCode::Forbidden => StatusCode::FORBIDDEN,
            ErrorCode::NotFound => StatusCode::NOT_FOUND,
            ErrorCode::PersistenceFailed | ErrorCode::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(self.0)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::{Method, Request},
    };
    use serde_json::Value;
    use tower::ServiceExt;

    fn test_app() -> Router {
        let repository = SqliteRepository::in_memory().expect("repository");
        let runtime = Runtime::restore("test-runtime", repository).expect("runtime");
        router(AppState::new(runtime, "disabled"))
    }

    async fn post_command(app: &Router, envelope: Value) -> Value {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v2/runtime/commands")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(envelope.to_string()))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        serde_json::from_slice(
            &to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body"),
        )
        .expect("JSON response")
    }

    #[tokio::test]
    async fn health_and_bootstrap_report_the_runtime() {
        let health = test_app()
            .oneshot(
                Request::get("/api/v2/health")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(health.status(), StatusCode::OK);
        let value: Value = serde_json::from_slice(
            &to_bytes(health.into_body(), usize::MAX)
                .await
                .expect("body"),
        )
        .expect("json");
        assert_eq!(value["status"], "ok");
        assert_eq!(value["protocol_version"], PROTOCOL_VERSION);
        assert_eq!(value["schema_version"], 4);

        let bootstrap = test_app()
            .oneshot(
                Request::get("/api/v2/runtime/bootstrap")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(bootstrap.status(), StatusCode::OK);

        for path in [
            "/api/v2/players",
            "/api/v2/history/sessions?limit=10",
            "/api/v2/statistics/players",
        ] {
            let response = test_app()
                .oneshot(Request::get(path).body(Body::empty()).expect("request"))
                .await
                .expect("response");
            assert_eq!(response.status(), StatusCode::OK, "{path}");
        }
    }

    #[tokio::test]
    async fn command_endpoint_starts_a_versioned_game() {
        let envelope = serde_json::json!({
            "protocol_version": 1,
            "command_id": "start-1",
            "runtime_instance_id": "test-runtime",
            "expected_revision": 0,
            "command": {
                "type": "start_game",
                "game_type": "countup",
                "player_ids": ["Ada"],
                "options": {"rounds": 8}
            }
        });
        let response = test_app()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v2/runtime/commands")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(envelope.to_string()))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let value: Value = serde_json::from_slice(
            &to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body"),
        )
        .expect("json");
        assert_eq!(value["revision"], 1);
        assert_eq!(value["state"]["game_type"], "count_up");
    }

    #[tokio::test]
    async fn command_endpoint_exposes_the_committed_session_state() {
        let envelope = serde_json::json!({
            "protocol_version": 1,
            "command_id": "session-1",
            "runtime_instance_id": "test-runtime",
            "expected_revision": 0,
            "command": {
                "type": "start_session",
                "session_id": "session-1",
                "players": [
                    {
                        "id": "ada",
                        "name": "Ada",
                        "avatar": "nova",
                        "color": "#ff00aa"
                    }
                ]
            }
        });
        let response = test_app()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v2/runtime/commands")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(envelope.to_string()))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let value: Value = serde_json::from_slice(
            &to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body"),
        )
        .expect("json");
        assert_eq!(value["revision"], 1);
        assert_eq!(value["session"]["screen"], "game_select");
        assert_eq!(value["session"]["players"][0]["id"], "ada");
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)] // Covers commands, history, replay and 404 as one API flow.
    async fn correction_commands_replay_x01_through_the_public_contract() {
        let app = test_app();
        let commands = [
            serde_json::json!({
                "type": "start_session",
                "session_id": "session-edit",
                "players": [{
                    "id": "ada", "name": "Ada", "avatar": "🦊", "color": "#ff00aa"
                }]
            }),
            serde_json::json!({
                "type": "prepare_game", "game_type": "x01",
                "options": {"start_score": 40, "out_rule": "double"}
            }),
            serde_json::json!({"type": "start_prepared_game", "game_id": "game-edit"}),
            serde_json::json!({"type": "mark_game_playing"}),
            serde_json::json!({
                "type": "ingest_dart",
                "event": {
                    "type": "hit", "seq": 1, "field": 20, "ring": "double",
                    "multiplier": 2, "label": "D20", "score": 40
                }
            }),
            serde_json::json!({
                "type": "correct_dart", "action_id": 1,
                "replacement": {
                    "type": "hit", "seq": 999, "field": 20, "ring": "single_inner",
                    "multiplier": 1, "label": "S20", "score": 20
                }
            }),
            serde_json::json!({"type": "delete_dart", "action_id": 1}),
        ];
        let mut result = Value::Null;
        for (revision, command) in commands.into_iter().enumerate() {
            result = post_command(
                &app,
                serde_json::json!({
                    "protocol_version": 1,
                    "command_id": format!("edit-{revision}"),
                    "runtime_instance_id": "test-runtime",
                    "expected_revision": revision,
                    "command": command
                }),
            )
            .await;
            if revision == 4 {
                assert_eq!(
                    result["state"]["state"]["editable_darts"][0]["action_id"],
                    1
                );
            }
            if revision == 5 {
                assert_eq!(
                    result["state"]["state"]["editable_darts"][0]["event"]["label"],
                    "S20"
                );
                assert_eq!(
                    result["state"]["state"]["editable_darts"][0]["event"]["seq"],
                    1
                );
            }
        }
        assert_eq!(result["revision"], 7);
        assert_eq!(result["session"]["screen"], "playing");
        assert_eq!(result["session"]["standings"][0]["session_points"], 0);
        assert_eq!(result["state"]["state"]["players"][0]["score"], 40);
        assert_eq!(result["state"]["state"]["darts_in_turn"], 0);

        for (path, assertion) in [
            ("/api/v2/history/sessions/session-edit", ("games", 1_usize)),
            ("/api/v2/history/games/game-edit", ("events", 3_usize)),
            (
                "/api/v2/history/games/game-edit/replay",
                ("events", 3_usize),
            ),
        ] {
            let response = app
                .clone()
                .oneshot(Request::get(path).body(Body::empty()).expect("request"))
                .await
                .expect("response");
            assert_eq!(response.status(), StatusCode::OK, "{path}");
            let value: Value = serde_json::from_slice(
                &to_bytes(response.into_body(), usize::MAX)
                    .await
                    .expect("body"),
            )
            .expect("history JSON");
            assert_eq!(
                value[assertion.0].as_array().map(Vec::len),
                Some(assertion.1),
                "{path}"
            );
        }

        let missing = app
            .oneshot(
                Request::get("/api/v2/history/games/missing")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
        let error: ContractError = serde_json::from_slice(
            &to_bytes(missing.into_body(), usize::MAX)
                .await
                .expect("body"),
        )
        .expect("error JSON");
        assert_eq!(error.code, ErrorCode::NotFound);
    }

    #[test]
    fn websocket_origin_must_match_host() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::HOST,
            HeaderValue::from_static("dartboard.local:8000"),
        );
        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("https://attacker.example"),
        );
        let error = validate_websocket_origin(&headers).expect_err("must reject");
        assert_eq!(error.code, ErrorCode::Forbidden);
    }

    #[tokio::test]
    async fn browser_command_origin_must_match_host() {
        let envelope = serde_json::json!({
            "protocol_version": 1,
            "command_id": "cross-origin",
            "runtime_instance_id": "test-runtime",
            "expected_revision": 0,
            "command": {"type": "undo"}
        });
        let response = test_app()
            .oneshot(
                Request::post("/api/v2/runtime/commands")
                    .header(header::HOST, "dartboard.local:8000")
                    .header(header::ORIGIN, "https://attacker.example")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(envelope.to_string()))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
