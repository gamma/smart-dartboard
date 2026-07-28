# Deployment auf Raspberry Pi / Linux

## Hardware-Empfehlung

Für reines BLE-Logging reicht theoretisch ein Raspberry Pi Zero W. Für eine spätere Beamer-/UI-Anwendung ist er aber knapp.

Empfehlung:

1. **Raspberry Pi Zero 2 W** als Minimalziel
   - deutlich schneller als Zero W
   - klein, stromsparend
   - BLE onboard
   - gut für Headless BLE-Backend

2. **Raspberry Pi 4 / 5 oder Mini-PC** für Beamer/UI
   - besser für Browser/Electron/Animationen
   - HDMI stabiler
   - mehr RAM/CPU

Pragmatische Architektur:

```text
Phase 1: Zero 2 W als BLE-Backend
Phase 2: Frontend über Websocket an Beamer-Rechner oder lokal auf größerem Pi
```

Wenn alles auf einem Gerät laufen soll inklusive Beamer-UI, lieber Pi 4/5 oder Mini-PC.

## Native Installation

```bash
sudo apt update
sudo apt install -y python3 python3-venv bluetooth bluez

cd smart-dartboard-project
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt

python scan_ble.py
python dump_gatt.py --name SDB-BT
python live_dartboard.py --name SDB-BT
```

## Docker Deployment

Bluetooth in Docker braucht Zugriff auf BlueZ/DBus. Der einfachste Ansatz ist privileged + host network:

```bash
docker compose up --build
```

Oder manuell:

```bash
docker build -t sdb-dartboard .
docker run --rm -it \
  --net=host \
  --privileged \
  -v /var/run/dbus:/var/run/dbus \
  sdb-dartboard
```

## Erwartete Ausgabe

```text
HIT S20 score=20 seq=...
HIT T20 score=60 seq=...
MISS seq=...
BUTTON menu press seq=...
BUTTON menu release seq=...
```

## Hinweise zu Linux/BlueZ

Falls Scan/Connect nicht geht:

```bash
sudo systemctl status bluetooth
sudo systemctl restart bluetooth
bluetoothctl show
rfkill list
sudo rfkill unblock bluetooth
```

Bei Raspberry Pi OS Lite muss Bluetooth aktiv sein. Keine parallele Verbindung mit LightBlue/SDBplay offen lassen, da BLE-Peripherals oft nur eine Verbindung gleichzeitig erlauben.
