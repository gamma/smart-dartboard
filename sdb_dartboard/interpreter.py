from __future__ import annotations

from typing import Any, Dict


class EventInterpreter:
    """Adds context to raw decoded packets.

    Important: code 00 00 ee means either miss or button release.
    We decide based on whether a button press is currently active.
    """

    def __init__(self) -> None:
        self.menu_button_down = False

    def interpret(self, decoded: Dict[str, Any]) -> Dict[str, Any]:
        typ = decoded.get("type")

        if typ == "button" and decoded.get("action") == "press":
            self.menu_button_down = True
            return decoded

        if typ == "button" and decoded.get("action") == "long_press":
            # Keep button_down true until following neutral/release.
            return decoded

        if typ == "neutral" and decoded.get("meaning") == "miss_or_button_release":
            if self.menu_button_down:
                self.menu_button_down = False
                return {**decoded, "type": "button", "button": "menu", "action": "release"}
            return {**decoded, "type": "miss", "score": 0, "label": "MISS"}

        return decoded
