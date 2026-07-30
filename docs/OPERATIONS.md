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

## Netzwerk- und Sicherheitsmodell

Die Anwendung ist für genau ein Dartboard in einem isolierten lokalen Netz
gedacht. Sie besitzt bewusst keine Benutzerkonten. Deshalb gelten für den
Betrieb diese Grenzen:

- Port `8000` nur im internen Dartboard-Netz freigeben und niemals per
  Router-Portfreigabe oder öffentlichem Reverse Proxy ins Internet stellen.
- Controller und Projektor aus demselben Origin öffnen, zum Beispiel beide
  über `http://dartboard.local:8000`. Fremde Browser-Origins und WebSockets
  werden abgewiesen.
- Das Board nicht gemeinsam mit einem unkontrollierten Gast-WLAN betreiben.
  Falls das unvermeidbar ist, den Zugriff per Host-Firewall auf die festen
  Controller-/Projektor-Adressen begrenzen.
- `SDB_ALLOW_TEST_EVENTS=0` im Produktivbetrieb beibehalten.
- Nach Möglichkeit `SDB_DEVICE_ADDRESS` auf die feste BLE-Adresse der echten
  Scheibe setzen. Dann verbindet sich der Dienst nicht allein anhand des
  Gerätenamens.

Der Container läuft ohne `privileged`, ohne Host-Netzwerk, ohne zusätzliche
Linux-Capabilities und mit `no-new-privileges`. Für BLE wird nur der
D-Bus-Socket des Hosts read-only eingebunden. Falls die lokale
BlueZ-Konfiguration den Zugriff darüber verweigert, sollte gezielt die
D-Bus-/BlueZ-Berechtigung angepasst werden; `privileged: true` ist nicht der
empfohlene Rückweg.

Docker Desktop auf macOS und Windows stellt dem Linux-Container den Bluetooth-
Stack des Hosts nicht als BlueZ-D-Bus zur Verfügung. Dort muss BLE für einen
reinen UI-Test deaktiviert werden:

```bash
SDB_ENABLE_BLE=0 SDB_ALLOW_TEST_EVENTS=1 \
  docker compose -f compose.production.yml up -d --wait --wait-timeout 60
```

Die Meldung `BLE connection loop error: [Errno 2] No such file or directory`
bedeutet, dass der BlueZ-D-Bus-Socket im Container nicht erreichbar ist. Der
HTTP-Dienst und der Healthcheck können dabei trotzdem laufen. Echte
BLE-Hardware im Container wird auf einem Linux-Host mit laufendem BlueZ und
eingebundenem `/var/run/dbus` unterstützt.

### Native Entwicklung auf macOS

Bleak verwendet nativ unter macOS CoreBluetooth. Die echte Scheibe kann deshalb
außerhalb von Docker direkt aus der Python-Entwicklungsumgebung angesprochen
werden:

```bash
./scripts/dev.sh --ble
```

Beim ersten Scan fragt macOS nach der Bluetoothberechtigung für das Terminal
oder den Python-Prozess. Falls die Berechtigung abgelehnt wurde, muss sie unter
**Systemeinstellungen → Datenschutz & Sicherheit → Bluetooth** aktiviert
werden. Andere Apps wie SDBplay müssen vom Board getrennt sein.

Eine feste `SDB_DEVICE_ADDRESS` ist unter macOS keine Bluetooth-MAC-Adresse,
sondern eine von CoreBluetooth vergebene und an diesen Mac gebundene UUID. Für
den ersten Start sollte die Adresse leer bleiben, damit das Board über
`SDB_DEVICE_NAME=SDB-BT` gefunden wird.

## Erstinstallation

1. Linux und BlueZ installieren und Bluetooth aktivieren.
2. Andere Apps wie SDBplay oder LightBlue vom Dartboard trennen.
3. Repository auschecken.
4. `.env.example` nach `.env` kopieren und mindestens Version sowie optionale
   feste Board-Adresse prüfen.
5. `docker compose -f compose.production.yml up -d --wait --wait-timeout 60`
   starten und den erfolgreichen Healthcheck abwarten.
6. `http://<host>:8000/control` auf dem Tablet öffnen.
7. `http://localhost:8000/projector` auf dem Projektorrechner öffnen.
8. Über **Projektor kalibrieren** das Board-Setup öffnen, die Scheibe
   deckungsgleich ausrichten, das gewünschte Artwork-Theme auswählen und dort
   **Projektor-Sound einschalten**.
9. Den **Testton** im Board-Setup auslösen und den Status `BEREIT` prüfen.

Ohne verbundenes BLE-Board läuft die Projektoransicht im Testmodus. In diesem
Modus erzeugt ein Klick auf ein Scheibensegment den entsprechenden Treffer;
für einen Fehlwurf steht daneben **MISS** bereit.

Auf dem Anleitungsbildschirm kann der Startspieler direkt angetippt oder über
**Zufall** ausgelost werden. Ohne Eingriff startet im ersten Spiel der zuerst
für die Session ausgewählte Spieler. Nach jedem gewerteten Spiel rotiert der
Start automatisch zum nächsten Session-Spieler. Ein abgebrochenes Spiel
verändert diese Rotation nicht. Der gewählte Startspieler wird auf Controller
und Projektor angezeigt.

Nach einem gewerteten Spiel startet ein doppelter Druck auf die
Spielerwechsel-Taste innerhalb von drei Sekunden eine Revanche mit demselben
Modus, denselben Optionen und rotierendem Startspieler. Der erste Druck zeigt
die Bestätigung auf Controller und Projektor; ein einzelner Druck verändert das
Ergebnis nicht.

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
automatischen Neustart. Ohne diese Option kann der Projektor-Browser die
automatische Audioausgabe blockieren. Der Status erscheint dann als
`AUTOPLAY BLOCKIERT` im Board-Setup auf dem Controller; auf dem Projektor
selbst gibt es bewusst keine Sound-Schaltfläche.

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
- vollständige Aktions- und Undo-Historie
- aktueller Screen
- Projektorkalibrierung

## Wurfkorrektur am Controller

Während eines Spiels zeigt der Controller ausschließlich die drei Dartplätze
des aktuell aktiven Spielers:

- Belegten Dart antippen, um ihn über die Scheibe zu ersetzen, als `MISS` zu
  werten oder zu löschen.
- Den nächsten freien Dartplatz antippen, um einen vom Board nicht erkannten
  Wurf manuell nachzutragen.
- Direkt nach einem Spielerwechsel nimmt `Letzte Aktion zurück` beim ersten
  Drücken nur den Wechsel zurück. Der vorherige Spieler wird wieder aktiv und
  alle drei Würfe bleiben sichtbar.
- Erst ein zweites `Letzte Aktion zurück` entfernt den letzten Dart dieses
  Spielers. Das ist bewusst ein Double-Back-Ablauf.
- Auf dem Ergebnisbildschirm kann der letzte Spielzug ebenfalls korrigiert
  werden. Sieger und Sessionpunkte werden danach neu berechnet.

Solange die Korrekturscheibe geöffnet ist, pausiert die BLE- und
Projektor-Testeingabe für höchstens 60 Sekunden. Manuell nachgetragene Würfe
bleiben mit der Quelle `manual` in Statistik und Replay erkennbar.

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
