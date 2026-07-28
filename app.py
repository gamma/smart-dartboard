from __future__ import annotations

import asyncio
import contextlib
import logging
import os
from contextlib import asynccontextmanager
from pathlib import Path
from typing import Any, Dict, List, Optional

from fastapi import FastAPI, HTTPException, WebSocket, WebSocketDisconnect
from fastapi.responses import FileResponse, HTMLResponse
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel, Field

from sdb_dartboard.client import SdbDartboardClient
from sdb_dartboard.game import GameEngine
from sdb_dartboard.session import EventPipeline, SessionController
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


async def publish_state(event: Optional[Dict[str, Any]] = None) -> None:
    experience = controller.public_state()
    await manager.broadcast_json(
        {
            "type": "experience",
            "experience": experience,
            # Compatibility with the original web client.
            "state": experience["game"],
            "event": event,
        }
    )


@asynccontextmanager
async def lifespan(_: FastAPI):
    global ble_task, ble_enabled
    ble_enabled = os.environ.get("SDB_ENABLE_BLE", "1").lower() not in {"0", "false"}
    if ble_enabled:
        client = SdbDartboardClient(
            name=os.environ.get("SDB_DEVICE_NAME", "SDB-BT"),
            address=os.environ.get("SDB_DEVICE_ADDRESS") or None,
        )

        async def on_event(event: Dict[str, Any]) -> None:
            accepted = await pipeline.process(event, source="ble")
            if accepted:
                await publish_state(event)

        ble_task = asyncio.create_task(client.run(on_event))
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


class NewGameRequest(BaseModel):
    game_type: str = "countup"
    players: List[str] = Field(default_factory=lambda: ["Player 1", "Player 2"])
    x01_start_score: int = 501


class PlayerRequest(BaseModel):
    name: str = Field(min_length=1, max_length=32)
    avatar: str = Field(default="comet", max_length=32)
    color: str = Field(default="#28e7ff", pattern=r"^#[0-9a-fA-F]{6}$")


class SessionStartRequest(BaseModel):
    player_ids: List[str] = Field(min_length=1, max_length=8)


class GamePrepareRequest(BaseModel):
    game_type: str
    options: Dict[str, Any] = Field(default_factory=dict)


class ScreenRequest(BaseModel):
    screen: str


class CalibrationRequest(BaseModel):
    corners: List[Dict[str, float]]
    scale: float = Field(default=1.0, ge=0.5, le=2.0)
    offset_x: float = Field(default=0.0, ge=-1.0, le=1.0)
    offset_y: float = Field(default=0.0, ge=-1.0, le=1.0)


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
    session = controller.start_session(req.player_ids)
    await publish_state({"type": "session_started", "session_id": session["id"]})
    return controller.public_state()


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


@app.post("/api/game/next")
async def next_game():
    controller.next_game()
    await publish_state({"type": "next_game"})
    return controller.public_state()


@app.post("/api/calibration")
async def save_calibration(req: CalibrationRequest):
    controller.save_calibration(req.model_dump())
    await publish_state({"type": "calibration_saved"})
    return controller.public_state()


# Compatibility endpoint for scripts and the original control interface.
@app.post("/api/new-game")
async def new_game(req: NewGameRequest):
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


@app.post("/api/event")
async def inject_event(event: Dict[str, Any]):
    allow = os.environ.get("SDB_ALLOW_TEST_EVENTS", "0").lower() in {"1", "true"}
    if ble_enabled and not allow:
        raise HTTPException(status_code=403, detail="Test events are disabled")
    await pipeline.process(event, source="test")
    await publish_state(event)
    return engine.state.as_dict()


@app.websocket("/ws")
async def ws_endpoint(websocket: WebSocket):
    await manager.connect(websocket)
    try:
        experience = controller.public_state()
        await websocket.send_json(
            {
                "type": "experience",
                "experience": experience,
                "state": experience["game"],
            }
        )
        while True:
            await websocket.receive_text()
    except WebSocketDisconnect:
        manager.disconnect(websocket)
