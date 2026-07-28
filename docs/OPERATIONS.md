# Betrieb in einer Spielhalle

## Zielaufbau

```text
SDB-BT Dartboard
       ↓ BLE
Raspberry Pi 4/5 oder Mini-PC
       ├── FastAPI + SQLite
       ├── Projektor-Browser im Kioskmodus
       └── lokales WLAN → Control-Tablet
```

Das System benötigt im Spielbetrieb keine Internetverbindung.

## Erstinstallation

1. Linux und BlueZ installieren und Bluetooth aktivieren.
2. Andere Apps wie SDBplay oder LightBlue vom Dartboard trennen.
3. Repository auschecken.
4. `docker compose up --build -d` starten.
5. `http://<host>:8000/control` auf dem Tablet öffnen.
6. `http://localhost:8000/projector` auf dem Projektorrechner öffnen.
7. Auf dem Projektor einmal **Sound aktivieren** wählen.
8. Über **Projektor kalibrieren** die Scheibe deckungsgleich ausrichten.

Die Kalibrierung und alle Spieldaten liegen in `./data/dartboard.db`.

## Projektor-Kioskmodus

Beispiel für Chromium:

```bash
chromium \
  --kiosk \
  --noerrdialogs \
  --disable-infobars \
  --autoplay-policy=no-user-gesture-required \
  http://localhost:8000/projector
```

`--autoplay-policy=no-user-gesture-required` erlaubt Sound nach einem
automatischen Neustart. Ohne diese Option verlangt der Browser einmalig eine
Interaktion über **Sound aktivieren**.

## Autostart

Docker Compose verwendet bereits `restart: unless-stopped`. Zusätzlich sollte
Docker beim Systemstart aktiviert sein:

```bash
sudo systemctl enable --now docker
```

Der Kiosk-Browser kann über den Desktop-Autostart oder einen eigenen
systemd-Service gestartet werden.

## Zustandswiederherstellung

Nach jedem Wurf und jeder Navigation wird ein vollständiger Checkpoint in
SQLite geschrieben. Nach Prozess-, Container- oder Rechnerneustart werden
folgende Daten wiederhergestellt:

- aktive Session und Teilnehmer
- ausgewählter Spielmodus und Optionen
- aktueller Spieler, Scores, Marks und Aufnahme
- Undo-Historie
- aktueller Screen
- Projektorkalibrierung

## Überwachung

```bash
curl http://localhost:8000/api/health
docker compose ps
docker compose logs --tail=100 dartboard
```

Beispiel:

```json
{
  "status": "ok",
  "database": "ok",
  "board": "connected",
  "ble_enabled": true
}
```

Bei laufendem Server, aber getrenntem Board lautet der Gesamtstatus
`degraded`. Control und Projektor zeigen dann `BOARD SUCHT` oder
`BOARD FEHLER`.

## Backup

Für ein konsistentes manuelles Backup zuerst den Container stoppen:

```bash
docker compose stop
cp data/dartboard.db data/dartboard-backup.db
docker compose start
```

Alternativ kann im laufenden Betrieb die SQLite-Backup-API verwendet werden.
Die Datenbank sollte regelmäßig auf ein anderes Speichermedium gesichert
werden.

## Störungsbehebung

### Board wird nicht gefunden

```bash
bluetoothctl show
rfkill list
sudo rfkill unblock bluetooth
sudo systemctl restart bluetooth
```

Danach prüfen, ob das Board noch mit einem Telefon verbunden ist.

### Oberfläche meldet OFFLINE

- FastAPI-/Docker-Prozess prüfen.
- Netzwerkverbindung des Tablets prüfen.
- `/api/health` direkt aufrufen.

### Oberfläche meldet BOARD SUCHT

Der Webserver funktioniert, aber BLE sucht oder verbindet noch. Board
einschalten, andere BLE-Verbindungen trennen und Containerlogs prüfen.

### Projektionsfläche stimmt nicht mehr

Wenn Scheibe oder Projektor mechanisch bewegt wurden, die Vierpunktkalibrierung
erneut durchführen. Bei unverändertem Aufbau wird das gespeicherte Profil
automatisch geladen.
