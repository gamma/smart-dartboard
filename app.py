from __future__ import annotations

import asyncio
import contextlib
import logging
from pathlib import Path
from typing import List, Optional

from fastapi import FastAPI, WebSocket, WebSocketDisconnect
from fastapi.responses import FileResponse, HTMLResponse
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel

from sdb_dartboard.client import SdbDartboardClient
from sdb_dartboard.game import GameEngine
from sdb_dartboard.ws import ConnectionManager

LOG = logging.getLogger(__name__)
ROOT = Path(__file__).resolve().parent
STATIC = ROOT / "web" / "static"

app = FastAPI(title="SDB Dartboard")
app.mount("/static", StaticFiles(directory=STATIC), name="static")

engine = GameEngine()
manager = ConnectionManager()
ble_task: Optional[asyncio.Task] = None
ble_enabled = False


class NewGameRequest(BaseModel):
    game_type: str = "countup"
    players: List[str] = ["Player 1", "Player 2"]
    x01_start_score: int = 501


async def publish_state(event=None):
    await manager.broadcast_json({"type": "state", "state": engine.state.as_dict(), "event": event})


@app.on_event("startup")
async def startup() -> None:
    global ble_task, ble_enabled
    import os
    ble_enabled = os.environ.get("SDB_ENABLE_BLE", "1") not in ("0", "false", "False")
    if ble_enabled:
        client = SdbDartboardClient(name=os.environ.get("SDB_DEVICE_NAME", "SDB-BT"), address=os.environ.get("SDB_DEVICE_ADDRESS") or None)

        async def on_event(event):
            state = engine.handle_event(event)
            await publish_state(event)

        ble_task = asyncio.create_task(client.run(on_event))
        LOG.info("BLE task started")
    else:
        LOG.info("BLE disabled via SDB_ENABLE_BLE=0")


@app.on_event("shutdown")
async def shutdown() -> None:
    global ble_task
    if ble_task:
        ble_task.cancel()
        with contextlib.suppress(Exception):
            await ble_task


@app.get("/")
async def root():
    return HTMLResponse('<meta http-equiv="refresh" content="0; url=/control">')


@app.get("/control")
async def control():
    return FileResponse(ROOT / "web" / "control.html")


@app.get("/projector")
async def projector():
    return FileResponse(ROOT / "web" / "projector.html")


@app.get("/api/state")
async def get_state():
    return engine.state.as_dict()


@app.post("/api/new-game")
async def new_game(req: NewGameRequest):
    engine.reset(req.game_type, req.players, req.x01_start_score)
    await publish_state({"type": "new_game"})
    return engine.state.as_dict()


@app.post("/api/next-player")
async def next_player():
    engine.next_player()
    await publish_state({"type": "next_player"})
    return engine.state.as_dict()


@app.post("/api/event")
async def inject_event(event: dict):
    # For development/testing without BLE.
    engine.handle_event(event)
    await publish_state(event)
    return engine.state.as_dict()


@app.websocket("/ws")
async def ws_endpoint(websocket: WebSocket):
    await manager.connect(websocket)
    try:
        await websocket.send_json({"type": "state", "state": engine.state.as_dict()})
        while True:
            await websocket.receive_text()
    except WebSocketDisconnect:
        manager.disconnect(websocket)
