use axum::{
    Json, Router,
    extract::{Path, State, WebSocketUpgrade, ws::Message},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use sdb_board::{
    BoardFailureCode, BoardIngress, BoardIngressOutcome, BoardPhase, BoardRejectReason, BoardStatus,
};
use sdb_companion::{
    COMPANION_PROTOCOL_VERSION, CompanionFrame, CompanionFrameKind, CompanionRole, PairedDevice,
    PairingAuthority, PairingBootstrap, PairingError, PairingGrant, PairingOffer, PairingRequest,
};
use sdb_contracts::{
    CommandEnvelope, ContractError, DartSource, Envelope, ErrorCode, MessageKind, PROTOCOL_VERSION,
    RuntimeCommand,
};
use sdb_game_core::{GameMetadata, registered_game_metadata};
use sdb_runtime::{CommandResult, Runtime, RuntimeSnapshot};
use sdb_storage::SqliteRepository;
use serde::{Deserialize, Serialize};
use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
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
    board: Arc<Mutex<BoardState>>,
    board_token: Option<Arc<str>>,
    companions: Arc<Mutex<PairingAuthority>>,
    companion_changes: broadcast::Sender<()>,
    companion_config: Option<Arc<CompanionConfig>>,
}

#[derive(Debug)]
struct CompanionConfig {
    host_id: String,
    certificate_sha256: String,
}

struct BoardState {
    status: BoardStatus,
    ingress: BoardIngress,
}

#[derive(Serialize)]
struct Health {
    status: &'static str,
    runtime: &'static str,
    database: &'static str,
    board: BoardPhase,
    board_failure_code: Option<BoardFailureCode>,
    companion: &'static str,
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

#[derive(Debug, Deserialize)]
struct BoardStatusRequest {
    phase: BoardPhase,
    failure_code: Option<BoardFailureCode>,
    detail: Option<String>,
    connection_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BoardPacketRequest {
    connection_id: String,
    raw_hex: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "disposition", rename_all = "snake_case")]
enum BoardPacketResponse {
    Applied { result: Box<CommandResult> },
    Duplicate,
    Button { button: String, action: String },
    Rejected { reason: BoardRejectReason },
    RuntimeRejected { error: ContractError },
}

#[derive(Debug, Serialize)]
struct CompanionDeviceView {
    device_id: String,
    device_name: String,
    role: CompanionRole,
    paired_at_ms: u64,
}

impl From<PairedDevice> for CompanionDeviceView {
    fn from(device: PairedDevice) -> Self {
        Self {
            device_id: device.device_id,
            device_name: device.device_name,
            role: device.role,
            paired_at_ms: device.paired_at_ms,
        }
    }
}

#[tokio::main]
async fn main() {
    let data_dir = PathBuf::from(env::var("SDB_DATA_DIR").unwrap_or_else(|_| "data".into()));
    std::fs::create_dir_all(&data_dir).expect("create data directory");
    let repository =
        SqliteRepository::open(data_dir.join("runtime.sqlite")).expect("open runtime database");
    let runtime = Runtime::restore(Uuid::new_v4().to_string(), repository)
        .expect("restore committed runtime");
    let ble_enabled = env_flag("SDB_ENABLE_BLE", false);
    let board_token = env::var("SDB_BOARD_TOKEN")
        .ok()
        .filter(|value| !value.trim().is_empty());
    assert!(
        !ble_enabled || board_token.is_some(),
        "SDB_BOARD_TOKEN must be set when SDB_ENABLE_BLE=1"
    );
    let port = env::var("SDB_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(8000);
    let bind_ip: IpAddr = env::var("SDB_BIND")
        .unwrap_or_else(|_| Ipv4Addr::UNSPECIFIED.to_string())
        .parse()
        .expect("SDB_BIND must be an IP address");
    let companion_config = companion_config(bind_ip);
    let state = AppState::new(runtime, ble_enabled, board_token, companion_config)
        .expect("restore companion device grants");
    let app = router(state);
    let address = SocketAddr::new(bind_ip, port);
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
    fn new(
        runtime: Runtime<SqliteRepository>,
        board_enabled: bool,
        board_token: Option<String>,
        companion_config: Option<CompanionConfig>,
    ) -> Result<Self, sdb_storage::StorageError> {
        let (states, _) = broadcast::channel(64);
        let (companion_changes, _) = broadcast::channel(16);
        let companion_devices = runtime.repository().companion_devices()?;
        Ok(Self {
            runtime: Arc::new(Mutex::new(runtime)),
            states,
            board: Arc::new(Mutex::new(BoardState {
                status: if board_enabled {
                    BoardStatus::unavailable()
                } else {
                    BoardStatus::disabled()
                },
                ingress: BoardIngress::new(),
            })),
            board_token: board_token.map(Arc::from),
            companions: Arc::new(Mutex::new(PairingAuthority::from_devices(
                companion_devices,
            ))),
            companion_changes,
            companion_config: companion_config.map(Arc::new),
        })
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
        .route("/api/v2/companion/pairing/open", post(open_pairing))
        .route("/api/v2/companion/pairing", post(pair_companion))
        .route("/api/v2/companion/devices", get(companion_devices))
        .route(
            "/api/v2/companion/devices/{device_id}",
            delete(revoke_companion),
        )
        .route(
            "/api/v2/companion/runtime/bootstrap",
            get(companion_bootstrap),
        )
        .route(
            "/api/v2/companion/runtime/events",
            get(companion_websocket),
        )
        .route("/api/v2/board/status", post(board_status))
        .route("/api/v2/board/packets", post(board_packet))
        .route("/api/v2/modes", get(modes))
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
    let board_status = state
        .board
        .lock()
        .map_err(|_| internal_error("board lock poisoned"))?
        .status
        .clone();
    let runtime = state
        .runtime
        .lock()
        .map_err(|_| internal_error("runtime lock poisoned"))?;
    let board_ready = board_status.phase.is_healthy();
    let schema_version = runtime
        .repository()
        .schema_version()
        .map_err(|_| internal_error("database schema query failed"))?;
    Ok(Json(Health {
        status: if board_ready { "ok" } else { "degraded" },
        runtime: "ok",
        database: "ok",
        board: board_status.phase,
        board_failure_code: board_status.failure_code,
        companion: if state.companion_config.is_some() {
            "ready"
        } else {
            "disabled"
        },
        protocol_version: PROTOCOL_VERSION,
        schema_version,
        revision: runtime.snapshot().revision,
    }))
}

async fn board_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<BoardStatusRequest>,
) -> Result<Json<BoardStatus>, ApiError> {
    authorize_board(&state, &headers)?;
    validate_status_request(&request)?;
    let mut board = state
        .board
        .lock()
        .map_err(|_| internal_error("board lock poisoned"))?;
    if !board.status.enabled {
        return Err(board_unavailable("board transport is disabled").into());
    }
    board.status = BoardStatus {
        enabled: true,
        phase: request.phase,
        failure_code: request.failure_code,
        detail: request.detail,
        connection_id: request.connection_id,
    };
    Ok(Json(board.status.clone()))
}

async fn board_packet(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<BoardPacketRequest>,
) -> Result<Json<BoardPacketResponse>, ApiError> {
    authorize_board(&state, &headers)?;
    validate_connection_id(&request.connection_id)?;
    let raw = strict_hex_packet(&request.raw_hex)?;
    let outcome = {
        let mut board = state
            .board
            .lock()
            .map_err(|_| internal_error("board lock poisoned"))?;
        if board.status.phase != BoardPhase::Ready
            || board.status.connection_id.as_deref() != Some(&request.connection_id)
        {
            return Err(board_unavailable("board transport is not ready").into());
        }
        board.ingress.ingest(&request.connection_id, &raw)
    };

    let response = match outcome {
        BoardIngressOutcome::Dart { event, command_id } => {
            let dispatch = {
                let mut runtime = state
                    .runtime
                    .lock()
                    .map_err(|_| internal_error("runtime lock poisoned"))?;
                let runtime_instance_id = runtime.instance_id().to_owned();
                runtime.dispatch_envelope(CommandEnvelope {
                    protocol_version: PROTOCOL_VERSION,
                    command_id: command_id.clone(),
                    runtime_instance_id,
                    expected_revision: None,
                    command: RuntimeCommand::IngestDart {
                        event,
                        source: DartSource::Board,
                    },
                })
            };
            match dispatch {
                Ok(result) => {
                    let message = state.snapshot(format!("{command_id}:state"))?;
                    let _ = state.states.send(message);
                    BoardPacketResponse::Applied {
                        result: Box::new(result),
                    }
                }
                Err(error)
                    if matches!(
                        error.code,
                        ErrorCode::InvalidCommand | ErrorCode::BoardUnavailable
                    ) =>
                {
                    BoardPacketResponse::RuntimeRejected { error }
                }
                Err(error) => return Err(error.into()),
            }
        }
        BoardIngressOutcome::Button { button, action } => {
            BoardPacketResponse::Button { button, action }
        }
        BoardIngressOutcome::Duplicate => BoardPacketResponse::Duplicate,
        BoardIngressOutcome::Rejected { reason } => BoardPacketResponse::Rejected { reason },
    };
    Ok(Json(response))
}

async fn bootstrap(State(state): State<AppState>) -> Result<Json<StateMessage>, ApiError> {
    Ok(Json(state.snapshot(Uuid::new_v4().to_string())?))
}

async fn snapshot(State(state): State<AppState>) -> Result<Json<StateMessage>, ApiError> {
    Ok(Json(state.snapshot(Uuid::new_v4().to_string())?))
}

async fn open_pairing(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<PairingBootstrap>, ApiError> {
    validate_same_origin(&headers)?;
    let config = companion_configured(&state)?;
    let offer = state
        .companions
        .lock()
        .map_err(|_| internal_error("companion lock poisoned"))?
        .open(now_ms())
        .map_err(pairing_error)?;
    Ok(Json(
        PairingBootstrap::new(
            config.host_id.clone(),
            config.certificate_sha256.clone(),
            offer,
        )
        .map_err(pairing_error)?,
    ))
}

async fn pair_companion(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<PairingRequest>,
) -> Result<Json<PairingGrant>, ApiError> {
    validate_same_origin(&headers)?;
    companion_configured(&state)?;
    let mut companions = state
        .companions
        .lock()
        .map_err(|_| internal_error("companion lock poisoned"))?;
    let grant = companions.pair(request, now_ms()).map_err(pairing_error)?;
    let device = companions
        .device(&grant.device_id)
        .cloned()
        .ok_or_else(|| internal_error("paired companion is missing"))?;
    let mut runtime = state
        .runtime
        .lock()
        .map_err(|_| internal_error("runtime lock poisoned"))?;
    if runtime
        .repository_mut()
        .save_companion_device(&device)
        .is_err()
    {
        let persisted = runtime
            .repository()
            .companion_devices()
            .map_err(|_| internal_error("companion persistence recovery failed"))?;
        *companions = PairingAuthority::from_devices(persisted);
        return Err(internal_error("companion grant persistence failed").into());
    }
    drop(runtime);
    let _ = state.companion_changes.send(());
    Ok(Json(grant))
}

async fn companion_devices(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<CompanionDeviceView>>, ApiError> {
    validate_same_origin(&headers)?;
    companion_configured(&state)?;
    let devices = state
        .companions
        .lock()
        .map_err(|_| internal_error("companion lock poisoned"))?
        .devices()
        .into_iter()
        .map(CompanionDeviceView::from)
        .collect();
    Ok(Json(devices))
}

async fn revoke_companion(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(device_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    validate_same_origin(&headers)?;
    companion_configured(&state)?;
    let mut companions = state
        .companions
        .lock()
        .map_err(|_| internal_error("companion lock poisoned"))?;
    if companions.device(&device_id).is_none() {
        return Err(not_found("companion device not found").into());
    }
    let mut runtime = state
        .runtime
        .lock()
        .map_err(|_| internal_error("runtime lock poisoned"))?;
    let revoked = runtime
        .repository_mut()
        .revoke_companion_device(&device_id, now_ms())
        .map_err(|_| internal_error("companion revocation persistence failed"))?;
    if !revoked || !companions.revoke(&device_id) {
        return Err(internal_error("companion grant state is inconsistent").into());
    }
    drop(runtime);
    let _ = state.companion_changes.send(());
    Ok(StatusCode::NO_CONTENT)
}

async fn companion_bootstrap(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<CompanionFrame>, ApiError> {
    companion_configured(&state)?;
    authorize_companion(&state, &headers)?;
    Ok(Json(companion_snapshot(&state)?))
}

async fn companion_websocket(
    State(state): State<AppState>,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> Result<Response, ApiError> {
    companion_configured(&state)?;
    let token = companion_token(&headers)?.to_owned();
    let states = state.states.subscribe();
    let companion_changes = state.companion_changes.subscribe();
    authenticate_companion_token(&state, &token)?;
    let initial = companion_snapshot(&state)?;
    Ok(upgrade.on_upgrade(move |socket| {
        stream_companion_states(
            socket,
            initial,
            states,
            companion_changes,
            state.companions,
            token,
        )
    }))
}

async fn modes() -> Json<Vec<GameMetadata>> {
    Json(registered_game_metadata().into_iter().copied().collect())
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
    let receiver = state.states.subscribe();
    let initial = state.snapshot(Uuid::new_v4().to_string())?;
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
    let runtime_instance_id = initial.runtime_instance_id;
    let mut revision = initial.revision;
    while let Ok(message) = receiver.recv().await {
        if message.runtime_instance_id != runtime_instance_id
            || message.revision > revision.saturating_add(1)
        {
            break;
        }
        if message.revision <= revision {
            continue;
        }
        if send_state(&mut socket, &message).await.is_err() {
            break;
        }
        revision = message.revision;
    }
    let _ = socket.send(Message::Close(None)).await;
}

async fn send_state(
    socket: &mut axum::extract::ws::WebSocket,
    state: &StateMessage,
) -> Result<(), axum::Error> {
    let json = serde_json::to_string(state).map_err(axum::Error::new)?;
    socket.send(Message::Text(json.into())).await
}

async fn stream_companion_states(
    mut socket: axum::extract::ws::WebSocket,
    initial: CompanionFrame,
    mut states: broadcast::Receiver<StateMessage>,
    mut companion_changes: broadcast::Receiver<()>,
    companions: Arc<Mutex<PairingAuthority>>,
    token: String,
) {
    if send_companion_frame(&mut socket, &initial).await.is_err() {
        return;
    }
    let runtime_instance_id = initial.runtime_instance_id.clone();
    let mut revision = initial.revision;
    loop {
        tokio::select! {
            state = states.recv() => {
                let Ok(state) = state else {
                    break;
                };
                if !companion_token_is_active(&companions, &token) {
                    break;
                }
                let Ok(frame) = companion_state_frame(state) else {
                    break;
                };
                if frame.runtime_instance_id != runtime_instance_id
                    || frame.revision > revision.saturating_add(1)
                {
                    break;
                }
                if frame.revision <= revision {
                    continue;
                }
                if send_companion_frame(&mut socket, &frame).await.is_err() {
                    return;
                }
                revision = frame.revision;
            }
            changed = companion_changes.recv() => {
                if changed.is_err() || !companion_token_is_active(&companions, &token) {
                    break;
                }
            }
        }
    }
    let _ = socket.send(Message::Close(None)).await;
}

async fn send_companion_frame(
    socket: &mut axum::extract::ws::WebSocket,
    frame: &CompanionFrame,
) -> Result<(), axum::Error> {
    let json = serde_json::to_string(frame).map_err(axum::Error::new)?;
    socket.send(Message::Text(json.into())).await
}

fn companion_snapshot(state: &AppState) -> Result<CompanionFrame, ContractError> {
    let runtime = state
        .runtime
        .lock()
        .map_err(|_| internal_error("runtime lock poisoned"))?;
    Ok(CompanionFrame {
        protocol_version: COMPANION_PROTOCOL_VERSION,
        runtime_instance_id: runtime.instance_id().to_owned(),
        revision: runtime.snapshot().revision,
        kind: CompanionFrameKind::Snapshot,
        payload: serde_json::to_value(runtime.snapshot())
            .map_err(|_| internal_error("companion snapshot serialization failed"))?,
    })
}

fn companion_state_frame(state: StateMessage) -> Result<CompanionFrame, ContractError> {
    Ok(CompanionFrame {
        protocol_version: COMPANION_PROTOCOL_VERSION,
        runtime_instance_id: state.runtime_instance_id,
        revision: state.revision,
        kind: CompanionFrameKind::State,
        payload: serde_json::to_value(state.payload)
            .map_err(|_| internal_error("companion state serialization failed"))?,
    })
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

fn authorize_board(state: &AppState, headers: &HeaderMap) -> Result<(), ContractError> {
    let Some(expected) = state.board_token.as_deref() else {
        return Err(forbidden("board ingress is not configured"));
    };
    let supplied = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or_default();
    if !constant_time_token_eq(expected, supplied) {
        return Err(forbidden("invalid board ingress token"));
    }
    Ok(())
}

fn authorize_companion(state: &AppState, headers: &HeaderMap) -> Result<(), ContractError> {
    authenticate_companion_token(state, companion_token(headers)?)
}

fn companion_configured(state: &AppState) -> Result<&CompanionConfig, ContractError> {
    state
        .companion_config
        .as_deref()
        .ok_or_else(|| forbidden("companion transport is disabled"))
}

fn companion_token(headers: &HeaderMap) -> Result<&str, ContractError> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|value| !value.is_empty())
        .ok_or_else(|| forbidden("missing companion token"))
}

fn authenticate_companion_token(state: &AppState, token: &str) -> Result<(), ContractError> {
    let companions = state
        .companions
        .lock()
        .map_err(|_| internal_error("companion lock poisoned"))?;
    if companions.authenticate(token).is_none() {
        return Err(forbidden("invalid companion token"));
    }
    Ok(())
}

fn companion_token_is_active(companions: &Mutex<PairingAuthority>, token: &str) -> bool {
    companions
        .lock()
        .is_ok_and(|authority| authority.authenticate(token).is_some())
}

fn constant_time_token_eq(expected: &str, supplied: &str) -> bool {
    let expected = expected.as_bytes();
    let supplied = supplied.as_bytes();
    let mut difference = expected.len() ^ supplied.len();
    for (index, expected_byte) in expected.iter().enumerate() {
        difference |= usize::from(*expected_byte ^ supplied.get(index).copied().unwrap_or(0));
    }
    difference == 0
}

fn validate_status_request(request: &BoardStatusRequest) -> Result<(), ContractError> {
    if request.phase == BoardPhase::Disabled {
        return Err(invalid_command("the gateway cannot disable board ingress"));
    }
    if let Some(detail) = &request.detail
        && detail.len() > 256
    {
        return Err(invalid_command("board status detail exceeds 256 bytes"));
    }
    if let Some(connection_id) = &request.connection_id {
        validate_connection_id(connection_id)?;
    }
    if request.phase == BoardPhase::Ready && request.connection_id.is_none() {
        return Err(invalid_command(
            "ready board status requires a connection id",
        ));
    }
    Ok(())
}

fn validate_connection_id(connection_id: &str) -> Result<(), ContractError> {
    if connection_id.is_empty()
        || connection_id.len() > 64
        || !connection_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(invalid_command("invalid board connection id"));
    }
    Ok(())
}

fn strict_hex_packet(raw_hex: &str) -> Result<Vec<u8>, ContractError> {
    if raw_hex.len() != 20 || !raw_hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(invalid_command(
            "board packet must contain exactly 20 hex digits",
        ));
    }
    (0..20)
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&raw_hex[index..index + 2], 16)
                .map_err(|_| invalid_command("board packet contains invalid hex"))
        })
        .collect()
}

fn env_flag(name: &str, default: bool) -> bool {
    env::var(name).map_or(default, |value| {
        !matches!(value.to_ascii_lowercase().as_str(), "0" | "false" | "no")
    })
}

fn companion_config(bind_ip: IpAddr) -> Option<CompanionConfig> {
    if !env_flag("SDB_ENABLE_COMPANION", false) {
        return None;
    }
    assert!(
        bind_ip.is_loopback(),
        "SDB_ENABLE_COMPANION=1 requires loopback SDB_BIND behind TLS termination"
    );
    let host_id = env::var("SDB_COMPANION_HOST_ID")
        .expect("SDB_COMPANION_HOST_ID must be set when companion transport is enabled");
    let certificate_sha256 = env::var("SDB_COMPANION_TLS_SHA256")
        .expect("SDB_COMPANION_TLS_SHA256 must be set when companion transport is enabled");
    PairingBootstrap::new(
        host_id.clone(),
        certificate_sha256.clone(),
        PairingOffer {
            code: "000000".into(),
            expires_at_ms: 0,
        },
    )
    .expect("companion identity configuration must be canonical");
    Some(CompanionConfig {
        host_id,
        certificate_sha256,
    })
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
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

fn invalid_command(message: &str) -> ContractError {
    ContractError {
        code: ErrorCode::InvalidCommand,
        message: message.into(),
        details: None,
    }
}

fn board_unavailable(message: &str) -> ContractError {
    ContractError {
        code: ErrorCode::BoardUnavailable,
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

fn pairing_error(error: PairingError) -> ContractError {
    match error {
        PairingError::InvalidCode | PairingError::AttemptsExhausted => {
            forbidden(&error.to_string())
        }
        PairingError::Closed
        | PairingError::Expired
        | PairingError::InvalidDevice
        | PairingError::InvalidIdentity => invalid_command(&error.to_string()),
        PairingError::EntropyUnavailable => internal_error(&error.to_string()),
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
    use futures_util::StreamExt;
    use serde_json::Value;
    use tokio_tungstenite::{connect_async, tungstenite::client::IntoClientRequest};
    use tower::ServiceExt;

    fn test_app() -> Router {
        let repository = SqliteRepository::in_memory().expect("repository");
        let runtime = Runtime::restore("test-runtime", repository).expect("runtime");
        router(AppState::new(runtime, false, None, None).expect("app state"))
    }

    fn test_companion_config() -> CompanionConfig {
        CompanionConfig {
            host_id: "test-host".into(),
            certificate_sha256: "ab".repeat(32),
        }
    }

    fn board_test_app() -> Router {
        let repository = SqliteRepository::in_memory().expect("repository");
        let runtime = Runtime::restore("test-runtime", repository).expect("runtime");
        router(
            AppState::new(runtime, true, Some("test-board-token".into()), None).expect("app state"),
        )
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

    async fn post_board(app: &Router, path: &str, payload: Value) -> (StatusCode, Value) {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(path)
                    .header(header::AUTHORIZATION, "Bearer test-board-token")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(payload.to_string()))
                    .expect("request"),
            )
            .await
            .expect("response");
        let status = response.status();
        let value = serde_json::from_slice(
            &to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body"),
        )
        .expect("JSON response");
        (status, value)
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
        assert_eq!(value["schema_version"], 6);
        assert_eq!(value["companion"], "disabled");

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
            "/api/v2/modes",
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
    async fn mode_metadata_exposes_all_native_registry_modes() {
        let response = test_app()
            .oneshot(
                Request::get("/api/v2/modes")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        let modes: Value = serde_json::from_slice(
            &to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body"),
        )
        .expect("mode metadata");
        assert_eq!(modes.as_array().map(Vec::len), Some(24));
        assert_cricket_metadata(&modes);
        assert!(modes.as_array().is_some_and(|items| {
            items
                .iter()
                .any(|mode| mode["slug"] == "countup" && mode["options"][0]["default"] == 8)
        }));
        assert!(modes.as_array().is_some_and(|items| {
            items.iter().any(|mode| {
                mode["slug"] == "eight_ball" && mode["min_players"] == 2 && mode["max_players"] == 2
            })
        }));
        assert!(modes.as_array().is_some_and(|items| {
            items.iter().any(|mode| {
                mode["slug"] == "heart_chase"
                    && mode["min_players"] == 2
                    && mode["max_players"] == 8
                    && mode["options"][0]["default"] == 3
            })
        }));
        assert!(modes.as_array().is_some_and(|items| {
            items.iter().any(|mode| {
                mode["slug"] == "target_rush"
                    && mode["ruleset_version"] == 2
                    && mode["options"][0]["default"] == 5
                    && mode["options"][1]["default"] == "normal"
            })
        }));
        assert!(modes.as_array().is_some_and(|items| {
            items.iter().any(|mode| {
                mode["slug"] == "ghost_chase"
                    && mode["ruleset_version"] == 2
                    && mode["artwork"] == "/static/assets/modes/ghost_chase.webp"
            })
        }));
        assert!(modes.as_array().is_some_and(|items| {
            items.iter().any(|mode| {
                mode["slug"] == "candy_cannon"
                    && mode["min_players"] == 2
                    && mode["artwork"] == "/static/assets/modes/candy_cannon.webp"
            })
        }));
        assert!(modes.as_array().is_some_and(|items| {
            items.iter().any(|mode| {
                mode["slug"] == "lightning_round"
                    && mode["ruleset_version"] == 2
                    && mode["options"][0]["default"] == 8
            })
        }));
        assert!(modes.as_array().is_some_and(|items| {
            items.iter().any(|mode| {
                mode["slug"] == "mini_golf"
                    && mode["ruleset_version"] == 2
                    && mode["options"][0]["default"] == 9
                    && mode["options"][1]["default"] == "normal"
            })
        }));
        assert!(modes.as_array().is_some_and(|items| {
            items.iter().any(|mode| {
                mode["slug"] == "simon_says"
                    && mode["ruleset_version"] == 2
                    && mode["options"][0]["default"] == 5
                    && mode["options"][1]["default"] == "easy"
            })
        }));
        assert!(modes.as_array().is_some_and(|items| {
            items.iter().any(|mode| {
                mode["slug"] == "robin_hood"
                    && mode["ruleset_version"] == 2
                    && mode["min_players"] == 2
                    && mode["options"][1]["default"] == "exact"
                    && mode["artwork"] == "/static/assets/modes/robin_hood.webp"
            })
        }));
        assert_block_drop_metadata(&modes);
        assert_boss_fight_metadata(&modes);
        assert_cookie_monster_metadata(&modes);
        assert_dart_sweeper_metadata(&modes);
        assert_darts_bingo_metadata(&modes);
        assert_dragon_eggs_metadata(&modes);
        assert_space_defender_metadata(&modes);
    }

    fn assert_block_drop_metadata(modes: &Value) {
        assert!(modes.as_array().is_some_and(|items| {
            items.iter().any(|mode| {
                mode["slug"] == "block_drop"
                    && mode["ruleset_version"] == 2
                    && mode["artwork"] == "/static/assets/modes/block_drop.webp"
                    && mode["options"].as_array().is_some_and(|options| {
                        options
                            .iter()
                            .map(|option| option["key"].as_str())
                            .collect::<Vec<_>>()
                            == [Some("difficulty"), Some("pace"), Some("drop_flow")]
                    })
                    && mode["control_legend"].as_array().is_some_and(|legend| {
                        legend.len() == 5
                            && legend.iter().any(|item| {
                                item["icon"] == "drop" && item["secondary_color"] == "#e76f51"
                            })
                    })
            })
        }));
    }

    fn assert_boss_fight_metadata(modes: &Value) {
        assert!(modes.as_array().is_some_and(|items| {
            items.iter().any(|mode| {
                mode["slug"] == "boss_fight"
                    && mode["ruleset_version"] == 1
                    && mode["artwork"] == "/static/assets/modes/boss_fight.webp"
                    && mode["options"].as_array().is_some_and(|options| {
                        options
                            .iter()
                            .map(|option| option["key"].as_str())
                            .collect::<Vec<_>>()
                            == [Some("boss_hp"), Some("weak_points"), Some("rounds")]
                    })
            })
        }));
    }

    fn assert_cricket_metadata(modes: &Value) {
        assert!(modes.as_array().is_some_and(|items| {
            items.iter().any(|mode| {
                mode["slug"] == "cricket"
                    && mode["artwork"] == "/static/assets/modes/cricket.webp"
                    && mode["instructions"]
                        .as_array()
                        .is_some_and(|steps| steps.len() == 4)
            })
        }));
    }

    fn assert_dragon_eggs_metadata(modes: &Value) {
        assert!(modes.as_array().is_some_and(|items| {
            items.iter().any(|mode| {
                mode["slug"] == "dragon_eggs"
                    && mode["ruleset_version"] == 2
                    && mode["artwork"] == "/static/assets/modes/dragon_eggs.webp"
                    && mode["options"][0]["default"] == 5
                    && mode["options"][1]["default"] == 4
                    && mode["instructions"]
                        .as_array()
                        .is_some_and(|steps| steps.len() == 3)
            })
        }));
    }

    fn assert_cookie_monster_metadata(modes: &Value) {
        assert!(modes.as_array().is_some_and(|items| {
            items.iter().any(|mode| {
                mode["slug"] == "cookie_monster"
                    && mode["ruleset_version"] == 2
                    && mode["artwork"] == "/static/assets/modes/cookie_monster.webp"
                    && mode["options"][0]["default"] == "easy"
                    && mode["options"][1]["default"] == 5
                    && mode["instructions"]
                        .as_array()
                        .is_some_and(|steps| steps.len() == 4)
            })
        }));
    }

    fn assert_dart_sweeper_metadata(modes: &Value) {
        assert!(modes.as_array().is_some_and(|items| {
            items.iter().any(|mode| {
                mode["slug"] == "dart_sweeper"
                    && mode["ruleset_version"] == 2
                    && mode["artwork"] == "/static/assets/modes/dart_sweeper.webp"
                    && mode["options"][0]["default"] == "classic"
                    && mode["min_players"] == 1
                    && mode["max_players"] == 8
                    && mode["instructions"]
                        .as_array()
                        .is_some_and(|steps| steps.len() == 3)
            })
        }));
    }

    fn assert_darts_bingo_metadata(modes: &Value) {
        assert!(modes.as_array().is_some_and(|items| {
            items.iter().any(|mode| {
                mode["slug"] == "darts_bingo"
                    && mode["ruleset_version"] == 2
                    && mode["artwork"] == "/static/assets/modes/darts_bingo.webp"
                    && mode["options"][0]["default"] == "line"
                    && mode["instructions"]
                        .as_array()
                        .is_some_and(|steps| steps.len() == 3)
            })
        }));
    }

    fn assert_space_defender_metadata(modes: &Value) {
        assert!(modes.as_array().is_some_and(|items| {
            items.iter().any(|mode| {
                mode["slug"] == "space_defender"
                    && mode["ruleset_version"] == 2
                    && mode["artwork"] == "/static/assets/modes/space_defender.webp"
                    && mode["options"][0]["default"] == 4
                    && mode["min_players"] == 1
                    && mode["max_players"] == 8
                    && mode["instructions"]
                        .as_array()
                        .is_some_and(|steps| steps.len() == 3)
            })
        }));
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
    async fn command_endpoint_starts_a_registered_cricket_game() {
        let envelope = serde_json::json!({
            "protocol_version": 1,
            "command_id": "start-cricket-1",
            "runtime_instance_id": "test-runtime",
            "expected_revision": 0,
            "command": {
                "type": "start_game",
                "game_type": "cricket",
                "player_ids": ["Ada", "Lin"],
                "options": {}
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
        assert_eq!(value["state"]["game_type"], "registered");
        assert_eq!(value["state"]["state"]["game_type"], "cricket");
        assert_eq!(value["state"]["state"]["ruleset_version"], 1);
        assert_eq!(
            value["state"]["state"]["overlay"]["cricket"]["remaining"]
                .as_array()
                .map(Vec::len),
            Some(7)
        );
    }

    #[tokio::test]
    async fn public_commands_correct_registered_cricket_by_action_id() {
        let app = test_app();
        let commands = [
            serde_json::json!({
                "type": "start_game", "game_type": "cricket",
                "player_ids": ["Ada", "Bob"], "options": {}
            }),
            serde_json::json!({
                "type": "ingest_dart",
                "event": {
                    "type": "hit", "seq": 1, "field": 20, "ring": "triple",
                    "multiplier": 3, "label": "T20", "score": 60
                }
            }),
            serde_json::json!({
                "type": "ingest_dart",
                "event": {
                    "type": "hit", "seq": 2, "field": 20, "ring": "triple",
                    "multiplier": 3, "label": "T20", "score": 60
                }
            }),
            serde_json::json!({
                "type": "correct_dart", "action_id": 2,
                "replacement": {"type": "miss", "seq": 999, "label": "MISS", "score": 0}
            }),
        ];
        let mut result = Value::Null;
        for (revision, command) in commands.into_iter().enumerate() {
            result = post_command(
                &app,
                serde_json::json!({
                    "protocol_version": 1,
                    "command_id": format!("cricket-edit-{revision}"),
                    "runtime_instance_id": "test-runtime",
                    "expected_revision": revision,
                    "command": command
                }),
            )
            .await;
        }

        assert_eq!(result["revision"], 4);
        assert_eq!(result["state"]["state"]["players"][0]["score"], 0);
        assert_eq!(
            result["state"]["state"]["editable_darts"][1]["event"]["seq"],
            2
        );
        assert_eq!(
            result["state"]["state"]["editable_darts"][1]["event"]["type"],
            "miss"
        );
    }

    #[tokio::test]
    async fn public_commands_correct_count_up_by_action_id() {
        let app = test_app();
        let commands = [
            serde_json::json!({
                "type": "start_game", "game_type": "countup",
                "player_ids": ["Ada"], "options": {"rounds": 5}
            }),
            serde_json::json!({
                "type": "ingest_dart",
                "event": {
                    "type": "hit", "seq": 1, "field": 20, "ring": "triple",
                    "multiplier": 3, "label": "T20", "score": 60
                }
            }),
            serde_json::json!({
                "type": "correct_dart", "action_id": 1,
                "replacement": {"type": "miss", "seq": 999, "label": "MISS", "score": 0}
            }),
        ];
        let mut result = Value::Null;
        for (revision, command) in commands.into_iter().enumerate() {
            result = post_command(
                &app,
                serde_json::json!({
                    "protocol_version": 1,
                    "command_id": format!("countup-edit-{revision}"),
                    "runtime_instance_id": "test-runtime",
                    "expected_revision": revision,
                    "command": command
                }),
            )
            .await;
        }

        assert_eq!(result["revision"], 3);
        assert_eq!(result["state"]["state"]["players"][0]["score"], 0);
        assert_eq!(
            result["state"]["state"]["editable_darts"][0]["event"]["seq"],
            1
        );
        assert_eq!(
            result["state"]["state"]["editable_darts"][0]["event"]["type"],
            "miss"
        );
    }

    #[tokio::test]
    async fn authenticated_board_packet_is_applied_once() {
        let app = board_test_app();
        let commands = [
            serde_json::json!({
                "type": "start_session",
                "session_id": "board-session",
                "players": [{
                    "id": "ada", "name": "Ada", "avatar": "🦊", "color": "#ff00aa"
                }]
            }),
            serde_json::json!({
                "type": "prepare_game", "game_type": "x01",
                "options": {"start_score": 40, "out_rule": "double"}
            }),
            serde_json::json!({"type": "start_prepared_game", "game_id": "board-game"}),
            serde_json::json!({"type": "mark_game_playing"}),
        ];
        for (revision, command) in commands.into_iter().enumerate() {
            post_command(
                &app,
                serde_json::json!({
                    "protocol_version": 1,
                    "command_id": format!("board-setup-{revision}"),
                    "runtime_instance_id": "test-runtime",
                    "expected_revision": revision,
                    "command": command
                }),
            )
            .await;
        }

        let (status, ready) = post_board(
            &app,
            "/api/v2/board/status",
            serde_json::json!({"phase": "ready", "connection_id": "link-1"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(ready["phase"], "ready");

        let packet = serde_json::json!({
            "connection_id": "link-1",
            "raw_hex": "0100000005000d00020f"
        });
        let (status, applied) = post_board(&app, "/api/v2/board/packets", packet.clone()).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(applied["disposition"], "applied");
        assert_eq!(applied["result"]["revision"], 5);
        assert_eq!(applied["result"]["session"]["screen"], "game_result");

        let (_, duplicate) = post_board(&app, "/api/v2/board/packets", packet).await;
        assert_eq!(duplicate["disposition"], "duplicate");

        let (_, rejected) = post_board(
            &app,
            "/api/v2/board/packets",
            serde_json::json!({
                "connection_id": "link-1",
                "raw_hex": "0200000005000d000200"
            }),
        )
        .await;
        assert_eq!(rejected["disposition"], "rejected");
        assert_eq!(rejected["reason"], "checksum");
    }

    #[tokio::test]
    async fn board_ingress_rejects_missing_auth_and_stale_connections() {
        let app = board_test_app();
        let unauthorized = app
            .clone()
            .oneshot(
                Request::post("/api/v2/board/status")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"phase":"scanning"}"#))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(unauthorized.status(), StatusCode::FORBIDDEN);

        post_board(
            &app,
            "/api/v2/board/status",
            serde_json::json!({"phase": "ready", "connection_id": "current-link"}),
        )
        .await;
        let (status, error) = post_board(
            &app,
            "/api/v2/board/packets",
            serde_json::json!({
                "connection_id": "old-link",
                "raw_hex": "0100000005000d00020f"
            }),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(error["code"], "board_unavailable");
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)] // Covers the complete one-time grant and live revocation flow.
    async fn companion_pairing_persists_bootstraps_and_revokes_projector_access() {
        let repository = SqliteRepository::in_memory().expect("repository");
        let runtime = Runtime::restore("test-runtime", repository).expect("runtime");
        let state =
            AppState::new(runtime, false, None, Some(test_companion_config())).expect("app state");
        let app = router(state.clone());

        let open = app
            .clone()
            .oneshot(
                Request::post("/api/v2/companion/pairing/open")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(open.status(), StatusCode::OK);
        let bootstrap: PairingBootstrap =
            serde_json::from_slice(&to_bytes(open.into_body(), usize::MAX).await.expect("body"))
                .expect("bootstrap");
        assert_eq!(bootstrap.host_id, "test-host");
        assert_eq!(bootstrap.certificate_sha256, "ab".repeat(32));

        let pair = app
            .clone()
            .oneshot(
                Request::post("/api/v2/companion/pairing")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "device_id": "ipad-projector",
                            "device_name": "Arcade iPad",
                            "code": bootstrap.offer.code
                        })
                        .to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(pair.status(), StatusCode::OK);
        let grant: PairingGrant =
            serde_json::from_slice(&to_bytes(pair.into_body(), usize::MAX).await.expect("body"))
                .expect("grant");
        assert_eq!(grant.role, CompanionRole::Projector);

        let devices = app
            .clone()
            .oneshot(
                Request::get("/api/v2/companion/devices")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        let device_json: Value = serde_json::from_slice(
            &to_bytes(devices.into_body(), usize::MAX)
                .await
                .expect("body"),
        )
        .expect("devices");
        assert_eq!(device_json[0]["device_id"], "ipad-projector");
        assert!(device_json[0].get("token_hash").is_none());

        let persisted = state
            .runtime
            .lock()
            .expect("runtime")
            .repository()
            .companion_devices()
            .expect("persisted devices");
        assert_eq!(persisted.len(), 1);
        assert_ne!(persisted[0].token_hash, grant.token);

        let bootstrap = app
            .clone()
            .oneshot(
                Request::get("/api/v2/companion/runtime/bootstrap")
                    .header(header::AUTHORIZATION, format!("Bearer {}", grant.token))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(bootstrap.status(), StatusCode::OK);
        let frame: CompanionFrame = serde_json::from_slice(
            &to_bytes(bootstrap.into_body(), usize::MAX)
                .await
                .expect("body"),
        )
        .expect("frame");
        assert_eq!(frame.kind, CompanionFrameKind::Snapshot);
        assert_eq!(frame.runtime_instance_id, "test-runtime");

        let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("listener");
        let address = listener.local_addr().expect("address");
        let server = tokio::spawn(axum::serve(listener, app.clone()).into_future());
        let mut request = format!("ws://{address}/api/v2/companion/runtime/events")
            .into_client_request()
            .expect("WebSocket request");
        request.headers_mut().insert(
            header::AUTHORIZATION,
            format!("Bearer {}", grant.token)
                .parse()
                .expect("authorization"),
        );
        let (mut projector, _) = connect_async(request).await.expect("connect projector");
        let initial: CompanionFrame = serde_json::from_str(
            projector
                .next()
                .await
                .expect("initial frame")
                .expect("initial message")
                .to_text()
                .expect("text frame"),
        )
        .expect("initial snapshot");
        assert_eq!(initial.kind, CompanionFrameKind::Snapshot);
        assert_eq!(initial.revision, 0);

        post_command(
            &app,
            serde_json::json!({
                "protocol_version": 1,
                "command_id": "companion-live-state",
                "runtime_instance_id": "test-runtime",
                "expected_revision": 0,
                "command": {
                    "type": "start_session",
                    "session_id": "companion-session",
                    "players": [{
                        "id": "ada", "name": "Ada", "avatar": "nova", "color": "#ff00aa"
                    }]
                }
            }),
        )
        .await;
        let live: CompanionFrame = serde_json::from_str(
            projector
                .next()
                .await
                .expect("live frame")
                .expect("live message")
                .to_text()
                .expect("text frame"),
        )
        .expect("live state");
        assert_eq!(live.kind, CompanionFrameKind::State);
        assert_eq!(live.revision, 1);

        let revoke = app
            .clone()
            .oneshot(
                Request::delete("/api/v2/companion/devices/ipad-projector")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(revoke.status(), StatusCode::NO_CONTENT);
        assert!(
            projector
                .next()
                .await
                .expect("revocation close")
                .expect("close message")
                .is_close()
        );

        let denied = app
            .oneshot(
                Request::get("/api/v2/companion/runtime/bootstrap")
                    .header(header::AUTHORIZATION, format!("Bearer {}", grant.token))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(denied.status(), StatusCode::FORBIDDEN);
        server.abort();
    }

    #[tokio::test]
    async fn companion_controller_routes_reject_cross_origin_browsers() {
        let repository = SqliteRepository::in_memory().expect("repository");
        let runtime = Runtime::restore("test-runtime", repository).expect("runtime");
        let app = router(
            AppState::new(runtime, false, None, Some(test_companion_config())).expect("app state"),
        );
        let response = app
            .oneshot(
                Request::post("/api/v2/companion/pairing/open")
                    .header(header::HOST, "dartboard.local:8000")
                    .header(header::ORIGIN, "https://attacker.example")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn companion_routes_are_closed_by_default() {
        let response = test_app()
            .oneshot(
                Request::post("/api/v2/companion/pairing/open")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let error: ContractError = serde_json::from_slice(
            &to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body"),
        )
        .expect("error");
        assert_eq!(error.code, ErrorCode::Forbidden);
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
    async fn setup_commands_persist_profiles_and_cancel_prepared_games() {
        let app = test_app();
        let commands = [
            serde_json::json!({
                "type":"create_player",
                "player":{"id":"ada","name":"Ada","avatar":"nova","color":"#ff00aa"}
            }),
            serde_json::json!({
                "type":"start_session",
                "session_id":"setup-session",
                "players":[{"id":"ada","name":"Ada","avatar":"nova","color":"#ff00aa"}]
            }),
            serde_json::json!({
                "type":"prepare_game",
                "game_type":"countup",
                "options":{"rounds":5}
            }),
            serde_json::json!({"type":"cancel_prepared_game"}),
        ];
        for (revision, command) in commands.into_iter().enumerate() {
            let result = post_command(
                &app,
                serde_json::json!({
                    "protocol_version":PROTOCOL_VERSION,
                    "command_id":format!("setup-{revision}"),
                    "runtime_instance_id":"test-runtime",
                    "expected_revision":revision,
                    "command":command,
                }),
            )
            .await;
            assert_eq!(result["revision"], revision + 1);
        }

        let response = app
            .clone()
            .oneshot(
                Request::get("/api/v2/players")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        let players: Value = serde_json::from_slice(
            &to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body"),
        )
        .expect("players");
        assert_eq!(players[0]["id"], "ada");

        let state = app
            .oneshot(
                Request::get("/api/v2/runtime/snapshot")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        let state: Value =
            serde_json::from_slice(&to_bytes(state.into_body(), usize::MAX).await.expect("body"))
                .expect("state");
        assert_eq!(
            state["payload"]["session"]["state"]["screen"],
            "game_select"
        );
        assert!(state["payload"]["session"]["state"]["prepared_game"].is_null());
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
