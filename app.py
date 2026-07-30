from __future__ import annotations

import asyncio
import contextlib
import logging
import os
import uuid
from contextlib import asynccontextmanager
from pathlib import Path
from typing import Any, Dict, List, Literal, Optional

from fastapi import FastAPI, HTTPException, Request, WebSocket, WebSocketDisconnect
from fastapi.responses import FileResponse, HTMLResponse, JSONResponse
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel, ConfigDict, Field, field_validator

from sdb_dartboard.client import SdbDartboardClient
from sdb_dartboard.game import GameEngine
from sdb_dartboard.session import EventPipeline, SessionController
from sdb_dartboard.validation import DartEventRequest, same_origin
from sdb_dartboard.ws import ConnectionManager

LOG = logging.getLogger(__name__)
ROOT = Path(__file__).resolve().parent
STATIC = ROOT / "web" / "static"
DATA_DIR = Path(os.environ.get("SDB_DATA_DIR", ROOT / "data"))

engine = GameEngine()
controller = SessionController(DATA_DIR / "dartboard.db", engine)
pipeline = EventPipeline(controller)
manager = ConnectionManager()
ble_task: Optional[asyncio.Task] = None
ble_enabled = False
SERVER_INSTANCE_ID = uuid.uuid4().hex
DEV_RELOAD = os.environ.get("SDB_DEV_RELOAD", "0").lower() in {"1", "true"}


async def publish_state(event: Optional[Dict[str, Any]] = None) -> None:
    experience = controller.public_state()
    await manager.broadcast_json(
        {
            "type": "experience",
            "experience": experience,
            "server_instance": SERVER_INSTANCE_ID,
            "dev_reload": DEV_RELOAD,
            # Compatibility with the original web client.
            "state": experience["game"],
            "event": event,
        }
    )


@asynccontextmanager
async def lifespan(_: FastAPI):
    global ble_task, ble_enabled
    ble_enabled = os.environ.get("SDB_ENABLE_BLE", "1").lower() not in {"0", "false"}
    test_events_allowed = (
        not ble_enabled
        or os.environ.get("SDB_ALLOW_TEST_EVENTS", "0").lower() in {"1", "true"}
    )
    controller.hardware = {
        "enabled": ble_enabled,
        "status": "starting" if ble_enabled else "disabled",
        "test_events": test_events_allowed,
    }
    if ble_enabled:
        client = SdbDartboardClient(
            name=os.environ.get("SDB_DEVICE_NAME", "SDB-BT"),
            address=os.environ.get("SDB_DEVICE_ADDRESS") or None,
        )

        async def on_event(event: Dict[str, Any]) -> None:
            accepted = await pipeline.process(event, source="ble")
            if accepted:
                await publish_state(event)

        async def on_status(status: Dict[str, Any]) -> None:
            controller.hardware = {**status, "test_events": test_events_allowed}
            await publish_state({"type": "hardware_status", **status})

        ble_task = asyncio.create_task(client.run(on_event, on_status))
        LOG.info("BLE task started")
    else:
        LOG.info("BLE disabled via SDB_ENABLE_BLE=0")
    try:
        yield
    finally:
        if ble_task:
            ble_task.cancel()
            with contextlib.suppress(asyncio.CancelledError):
                await ble_task
        controller.close()


app = FastAPI(title="SDB Dartboard", lifespan=lifespan)
app.mount("/static", StaticFiles(directory=STATIC), name="static")


@app.middleware("http")
async def protect_browser_mutations(request: Request, call_next):
    if request.method not in {"GET", "HEAD", "OPTIONS"} and not same_origin(
        request.headers.get("origin"), request.headers.get("host")
    ):
        response = JSONResponse(
            status_code=403,
            content={"detail": "Cross-origin control requests are not allowed"},
        )
    else:
        response = await call_next(request)
    response.headers["Content-Security-Policy"] = (
        "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; "
        "script-src 'self'; connect-src 'self' ws: wss:; frame-ancestors 'none'; "
        "base-uri 'none'; form-action 'self'"
    )
    response.headers["X-Content-Type-Options"] = "nosniff"
    response.headers["Referrer-Policy"] = "no-referrer"
    return response


class NewGameRequest(BaseModel):
    game_type: str = "countup"
    players: List[str] = Field(
        default_factory=lambda: ["Player 1", "Player 2"],
        min_length=1,
        max_length=8,
    )
    x01_start_score: int = Field(default=501)

    @field_validator("players")
    @classmethod
    def validate_player_names(cls, players: List[str]) -> List[str]:
        cleaned = [name.strip() for name in players]
        if any(not name or len(name) > 32 for name in cleaned):
            raise ValueError("Player names must contain 1–32 characters")
        return cleaned


class PlayerRequest(BaseModel):
    name: str = Field(min_length=1, max_length=32)
    avatar: str = Field(default="comet", max_length=32)
    color: str = Field(default="#28e7ff", pattern=r"^#[0-9a-fA-F]{6}$")


class SessionStartRequest(BaseModel):
    player_ids: List[str] = Field(min_length=1, max_length=8)
    language: Literal["de", "en"] = "de"


class GamePrepareRequest(BaseModel):
    game_type: str
    options: Dict[str, Any] = Field(default_factory=dict)


class GameActionRequest(BaseModel):
    action: str
    payload: Dict[str, Any] = Field(default_factory=dict)


class ScreenRequest(BaseModel):
    screen: str


class CalibrationPoint(BaseModel):
    model_config = ConfigDict(extra="forbid")
    x: float = Field(ge=0.0, le=1.0)
    y: float = Field(ge=0.0, le=1.0)


class CalibrationRequest(BaseModel):
    corners: List[CalibrationPoint] = Field(min_length=4, max_length=4)
    scale: float = Field(default=1.0, ge=0.5, le=2.0)
    offset_x: float = Field(default=0.0, ge=-1.0, le=1.0)
    offset_y: float = Field(default=0.0, ge=-1.0, le=1.0)


class ProjectorGeometryRequest(BaseModel):
    width: int = Field(ge=320, le=16384)
    height: int = Field(ge=240, le=16384)


class SoundSettingsRequest(BaseModel):
    enabled: bool


class SoundStatusRequest(BaseModel):
    status: str = Field(pattern=r"^(ready|blocked|unavailable)$")


class ArtThemeRequest(BaseModel):
    theme: str = Field(pattern=r"^(cartoon|neon)$")


class ThrowCorrectionRequest(BaseModel):
    turn_index: Optional[int] = Field(default=None, ge=0, le=2)
    action_id: Optional[int] = Field(default=None, ge=1)
    event: DartEventRequest


class ThrowDeleteRequest(BaseModel):
    action_id: int = Field(ge=1)


class CorrectionLockRequest(BaseModel):
    enabled: bool


@app.exception_handler(ValueError)
async def value_error_handler(_, exc: ValueError):
    from fastapi.responses import JSONResponse

    return JSONResponse(status_code=400, content={"detail": str(exc)})


@app.get("/")
async def root():
    return HTMLResponse('<meta http-equiv="refresh" content="0; url=/control">')


@app.get("/control")
async def control():
    return FileResponse(ROOT / "web" / "control.html")


@app.get("/projector")
async def projector():
    return FileResponse(ROOT / "web" / "projector.html")


@app.get("/api/bootstrap")
async def bootstrap():
    return controller.public_state()


@app.get("/api/health")
async def health():
    database_ok = controller.store.ping()
    board_status = controller.hardware.get("status", "unknown")
    board_ok = not ble_enabled or board_status == "connected"
    return {
        "status": "ok" if database_ok and board_ok else "degraded",
        "database": "ok" if database_ok else "error",
        "board": board_status,
        "ble_enabled": ble_enabled,
    }


@app.get("/api/state")
async def get_state():
    return engine.state.as_dict()


@app.post("/api/navigation/players")
async def show_players():
    controller.show_player_selection()
    await publish_state({"type": "navigation", "screen": "players"})
    return controller.public_state()


@app.post("/api/navigation")
async def navigate(req: ScreenRequest):
    controller.set_screen(req.screen)
    await publish_state({"type": "navigation", "screen": req.screen})
    return controller.public_state()


@app.post("/api/players")
async def create_player(req: PlayerRequest):
    player = controller.create_player(req.name, req.avatar, req.color)
    await publish_state({"type": "player_created", "player_id": player["id"]})
    return player


@app.post("/api/session/start")
async def start_session(req: SessionStartRequest):
    session = controller.start_session(req.player_ids, req.language)
    await publish_state({"type": "session_started", "session_id": session["id"]})
    return controller.public_state()


@app.get("/api/history/sessions")
async def history_sessions(limit: int = 50):
    return {"sessions": controller.store.list_sessions(limit)}


@app.get("/api/history/sessions/{session_id}")
async def history_session(session_id: str):
    detail = controller.store.session_detail(session_id)
    if detail is None:
        raise HTTPException(status_code=404, detail="Session not found")
    return detail


@app.get("/api/history/games/{game_id}")
async def history_game(game_id: str):
    detail = controller.store.game_detail(game_id)
    if detail is None:
        raise HTTPException(status_code=404, detail="Game not found")
    return detail


@app.get("/api/history/games/{game_id}/replay")
async def history_game_replay(game_id: str):
    replay = controller.store.game_replay(game_id)
    if replay is None:
        raise HTTPException(status_code=404, detail="Game not found")
    return replay


@app.get("/api/statistics/players")
async def player_statistics(include_test: bool = False):
    return {
        "players": controller.store.statistics(
            completed_only=True,
            include_nonproduction=include_test,
        )
    }


@app.get("/api/statistics/heatmap")
async def statistics_heatmap(
    player_id: Optional[str] = None,
    session_id: Optional[str] = None,
    game_type: Optional[str] = None,
    include_test: bool = False,
):
    return controller.store.heatmap(
        player_id=player_id,
        session_id=session_id,
        game_type=game_type,
        include_nonproduction=include_test,
    )


@app.get("/api/statistics/modes")
async def mode_statistics(include_test: bool = False):
    return {
        "modes": controller.store.mode_statistics(
            include_nonproduction=include_test
        )
    }


@app.get("/api/training/{player_id}/recommendations")
async def training_recommendations(player_id: str):
    if not any(
        player["id"] == player_id for player in controller.store.list_players()
    ):
        raise HTTPException(status_code=404, detail="Player not found")
    return controller.store.training_recommendations(player_id)


@app.get("/api/data/export")
async def export_data():
    return JSONResponse(
        content=controller.store.export_data(),
        headers={
            "Content-Disposition": (
                'attachment; filename="smart-dartboard-history.json"'
            )
        },
    )


@app.post("/api/session/end")
async def end_session():
    controller.end_session()
    await publish_state({"type": "session_finished"})
    return controller.public_state()


@app.post("/api/session/close")
async def close_session():
    controller.reset_to_attract()
    await publish_state({"type": "session_closed"})
    return controller.public_state()


@app.post("/api/game/prepare")
async def prepare_game(req: GamePrepareRequest):
    controller.prepare_game(req.game_type, req.options)
    await publish_state({"type": "game_prepared", "game_type": req.game_type})
    return controller.public_state()


@app.post("/api/game/start")
async def start_game():
    controller.start_game()
    await publish_state({"type": "countdown"})
    return controller.public_state()


@app.post("/api/game/live")
async def game_live():
    controller.set_screen("playing")
    await publish_state({"type": "game_live"})
    return controller.public_state()


@app.post("/api/game/action")
async def game_action(req: GameActionRequest):
    controller.game_action(req.action, req.payload)
    await publish_state({"type": "game_action", "action": req.action, "payload": req.payload})
    return controller.public_state()


@app.post("/api/game/next")
async def next_game():
    controller.next_game()
    await publish_state({"type": "next_game"})
    return controller.public_state()


@app.post("/api/game/abort")
async def abort_game():
    controller.abort_game()
    await publish_state({"type": "game_aborted"})
    return controller.public_state()


@app.post("/api/calibration")
async def save_calibration(req: CalibrationRequest):
    controller.save_calibration(req.model_dump())
    await publish_state({"type": "calibration_saved"})
    return controller.public_state()


@app.post("/api/calibration/reset")
async def reset_calibration():
    controller.reset_calibration()
    await publish_state({"type": "calibration_reset"})
    return controller.public_state()


@app.post("/api/projector/geometry")
async def projector_geometry(req: ProjectorGeometryRequest):
    controller.report_projector_geometry(req.width, req.height)
    await publish_state({"type": "projector_geometry"})
    return controller.public_state()


@app.post("/api/sound/settings")
async def sound_settings(req: SoundSettingsRequest):
    controller.set_sound_enabled(req.enabled)
    await publish_state({"type": "sound_settings", "enabled": req.enabled})
    return controller.public_state()


@app.post("/api/sound/status")
async def sound_status(req: SoundStatusRequest):
    controller.report_sound_status(req.status)
    await publish_state({"type": "sound_status", "status": req.status})
    return controller.public_state()


@app.post("/api/sound/test")
async def sound_test():
    sequence = int(asyncio.get_running_loop().time() * 1000)
    await publish_state({"type": "sound_test", "seq": sequence})
    return controller.public_state()


@app.post("/api/art-theme")
async def art_theme(req: ArtThemeRequest):
    controller.set_art_theme(req.theme)
    await publish_state({"type": "art_theme", "theme": req.theme})
    return controller.public_state()


# Compatibility endpoint for scripts and the original control interface.
@app.post("/api/new-game")
async def new_game(req: NewGameRequest):
    if controller.session_id or controller.game_id:
        raise HTTPException(
            status_code=409,
            detail="Standalone new-game cannot replace an active session",
        )
    engine.reset(
        req.game_type,
        req.players,
        req.x01_start_score,
        {"start_score": req.x01_start_score} if req.game_type == "x01" else {},
    )
    controller.screen = "playing"
    controller.selected_mode = req.game_type
    controller._persist()
    await publish_state({"type": "new_game"})
    return engine.state.as_dict()


@app.post("/api/next-player")
async def next_player():
    controller.next_player()
    await publish_state({"type": "next_player"})
    return engine.state.as_dict()


@app.post("/api/continue")
async def continue_turn():
    controller.continue_turn()
    await publish_state({"type": "continue"})
    return engine.state.as_dict()


@app.post("/api/undo")
async def undo():
    controller.undo()
    await publish_state({"type": "undo"})
    return engine.state.as_dict()


@app.post("/api/throw/correct")
async def correct_throw(req: ThrowCorrectionRequest):
    replacement = req.event.normalized()
    if req.action_id is not None:
        controller.correct_throw(req.action_id, replacement)
    elif req.turn_index is not None:
        controller.correct_turn_throw(req.turn_index, replacement)
    else:
        raise HTTPException(
            status_code=422,
            detail="action_id or turn_index is required",
        )
    correction_event = {
        "type": "throw_corrected",
        "action_id": req.action_id,
        "turn_index": req.turn_index,
        "replacement": replacement,
    }
    await publish_state(correction_event)
    return engine.state.as_dict()


@app.post("/api/throw/delete")
async def delete_throw(req: ThrowDeleteRequest):
    controller.delete_throw(req.action_id)
    await publish_state(
        {"type": "throw_deleted", "action_id": req.action_id}
    )
    return engine.state.as_dict()


@app.post("/api/throw/manual")
async def manual_throw(req: DartEventRequest):
    event = req.normalized()
    controller.manual_throw(event)
    await publish_state({**event, "source": "manual"})
    return engine.state.as_dict()


@app.post("/api/correction/lock")
async def correction_lock(req: CorrectionLockRequest):
    controller.set_correction_lock(req.enabled)
    await publish_state(
        {"type": "correction_lock", "enabled": req.enabled}
    )
    return controller.public_state()


@app.post("/api/event")
async def inject_event(req: DartEventRequest):
    allow = os.environ.get("SDB_ALLOW_TEST_EVENTS", "0").lower() in {"1", "true"}
    if ble_enabled and not allow:
        raise HTTPException(status_code=403, detail="Test events are disabled")
    event = req.normalized()
    accepted = await pipeline.process(event, source="test")
    if accepted:
        await publish_state(event)
    return engine.state.as_dict()


@app.websocket("/ws")
async def ws_endpoint(websocket: WebSocket):
    if not same_origin(websocket.headers.get("origin"), websocket.headers.get("host")):
        await websocket.close(code=1008, reason="Cross-origin WebSocket denied")
        return
    await manager.connect(websocket)
    try:
        experience = controller.public_state()
        await websocket.send_json(
            {
                "type": "experience",
                "experience": experience,
                "server_instance": SERVER_INSTANCE_ID,
                "dev_reload": DEV_RELOAD,
                "state": experience["game"],
            }
        )
        while True:
            await websocket.receive_text()
    except WebSocketDisconnect:
        manager.disconnect(websocket)
