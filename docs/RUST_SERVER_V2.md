# Rust Headless Server API v2

Stand: 2026-08-01

Der Rust-Server ist der parallele Migrationspfad für Linux und Docker. Er
ersetzt die produktive Python-API noch nicht. Die bestehende UI wird weiterhin
vom Python-Container bedient, bis Sessions, alle Modi, Statistiken und BLE
paritätisch portiert sind. Der Vorschau-Server liefert deshalb absichtlich noch
keine scheinbar funktionsfähigen Control-/Projector-Seiten aus.

## Start

Lokal:

```bash
SDB_ENABLE_BLE=0 SDB_DATA_DIR=./data-rust cargo run -p sdb-server
```

Als Vorschaucontainer auf Port 8001:

```bash
docker compose -f compose.rust.yml up --build
curl http://127.0.0.1:8001/api/v2/health
```

Der Container läuft ohne Root, ohne `privileged` und ohne Linux-Capabilities.
`/data` enthält `runtime.sqlite`.

Mit echter BLE-Scheibe unter Linux läuft Bleak als separater, unprivilegierter
Adapter. Zuerst ein zufälliges gemeinsames Secret in `.env` eintragen:

```dotenv
SDB_BOARD_TOKEN=<Ausgabe von: openssl rand -hex 32>
SDB_DEVICE_NAME=SDB-BT
SDB_DEVICE_ADDRESS=
```

Dann Runtime und Gateway gemeinsam starten:

```bash
docker compose -f compose.rust.yml -f compose.rust.ble.yml up --build
```

Das Overlay bindet ausschließlich den Linux-Systembus `/run/dbus` read-only
ein. Es benötigt weder `privileged` noch zusätzliche Linux-Capabilities. Docker
Desktop auf macOS reicht echte CoreBluetooth-Geräte nicht an Linux-Container
durch; dort bleibt das Overlay aus und der spätere native Apple-Adapter ist der
Produktionspfad.

## Endpunkte

| Methode | Pfad | Zweck |
| --- | --- | --- |
| `GET` | `/api/v2/health` | Runtime-, Datenbank-, Board-, Protokoll- und Schemastatus |
| `GET` | `/api/v2/runtime/bootstrap` | vollständiger versionierter Snapshot |
| `GET` | `/api/v2/runtime/snapshot` | erneuter Snapshot nach Lücke oder Reconnect |
| `POST` | `/api/v2/runtime/commands` | ein `CommandEnvelope` atomar anwenden |
| `GET` | `/api/v2/runtime/events` | WebSocket mit initialem und folgenden Snapshots |
| `POST` | `/api/v2/companion/pairing/open` | fünfminütiges Einmalcode-Fenster am Controller öffnen |
| `POST` | `/api/v2/companion/pairing` | Code einmalig gegen einen Projector-Grant tauschen |
| `GET` | `/api/v2/companion/devices` | gekoppelte Projector-Geräte ohne Token-Hashes auflisten |
| `DELETE` | `/api/v2/companion/devices/{device_id}` | Projector-Grant persistent widerrufen |
| `GET` | `/api/v2/companion/runtime/bootstrap` | authentisierter Projector-Vollsnapshot |
| `GET` | `/api/v2/companion/runtime/events` | authentisierter Projector-WebSocket ab Vollsnapshot |
| `POST` | `/api/v2/board/status` | interner, authentisierter Gateway-Status |
| `POST` | `/api/v2/board/packets` | interner, authentisierter FFF1-Rohpaket-Ingress |
| `GET` | `/api/v2/players` | persistente Spielerprofile |
| `GET` | `/api/v2/history/sessions` | Sessionhistorie; optional `?limit=` |
| `GET` | `/api/v2/history/sessions/{session_id}` | Session, Teilnehmer und enthaltene Spiele |
| `GET` | `/api/v2/history/games/{game_id}` | Spiel, kanonische Würfe und vollständige Auditkette |
| `GET` | `/api/v2/history/games/{game_id}/replay` | Initial-/Finalzustand und alle Replay-Frames einschließlich Korrekturen |
| `GET` | `/api/v2/statistics/players` | Langzeitstatistik aus gewerteten Produktionsspielen |

Browser-POSTs und WebSockets akzeptieren nur dieselbe Origin. Clients ohne
`Origin`, etwa lokale Diagnosewerkzeuge, bleiben möglich. Unvereinbare
Protokollversionen, falsche Runtime-IDs und veraltete Revisionen liefern stabile
Fehlercodes und passende HTTP-Statuscodes.

Die beiden Board-Endpunkte sind keine Browser-API. Sie verlangen
`Authorization: Bearer <SDB_BOARD_TOKEN>`. Bei aktiviertem BLE startet der
Server ohne Token nicht. `connection_id` bindet Pakete an genau eine
Transportverbindung; veraltete Links, falsche Paketlänge und ungültige
Checksummen ändern den Spielzustand nicht.

Die Controller-seitigen Pairing- und Geräte-Endpunkte unterliegen ebenfalls
dem Same-Origin-Schutz. Ein nativer Companion ohne `Origin` darf nur den
Einmalcode einlösen. Bootstrap und Live-Stream verlangen danach
`Authorization: Bearer <companion-token>` und liefern ausschließlich die
Projector-Rolle. Der Klartext-Token wird nur in der einmaligen Pairing-Antwort
ausgegeben. Eine Revisionslücke schließt den Stream und erzwingt einen neuen
Vollsnapshot; ein Widerruf schließt auch einen bereits verbundenen Socket.

Der Rust-Host stellt derzeit selbst kein Zertifikat aus. Companion-Zugriff ist
daher bis zum nativen TLS-Adapter beziehungsweise zu einer korrekt
konfigurierten HTTPS/WSS-Terminierung nur ein lokaler Entwicklungsbaustein und
kein freigegebener Klartext-LAN-Produktpfad. Bearer-Tokens gehören weder in
Query-Strings noch in Logs.

## Aktueller Funktionsumfang

- CountUp und X01 starten,
- Session mit vollständigen Spielerreferenzen starten, Modus vorbereiten und
  Startspieler festlegen,
- Countdown, Spiel, Ergebnis, nächste Spielauswahl, Rematch und
  Sessionzusammenfassung als gemeinsamen Screenfluss führen,
- Einzel- und Koop-Siege mit drei Sessionpunkten je Gewinner werten;
  Unentschieden und Abbrüche bleiben punktlos,
- kanonische Dart-Events übernehmen,
- rohe FFF1-Notifications über den Linux-Bleak-Sidecar begrenzt puffern, im
  gemeinsamen Rust-Ingress decodieren und pro Verbindung deduplizieren,
- die Wurfquelle transportneutral als `board`, `projector_test` oder
  `manual_correction` führen; ein Projektor-Testwurf markiert das gesamte Spiel
  als Test und schließt es aus der Standardstatistik aus,
- Turn fortsetzen und Undo; ein Undo des Siegtreffers öffnet zugleich das Spiel
  wieder und nimmt die Sessionwertung atomar zurück,
- X01-Würfe über stabile Action-IDs korrigieren oder löschen. Der Core bewahrt
  die ursprüngliche Sequenznummer, spielt alle späteren Aktionen neu ab und
  veröffentlicht die letzten zwei editierbaren Aufnahmen im Game-State,
- `command_id` deduplizieren,
- Commit und Snapshot in einer SQLite-Transaktion sichern,
- jedes akzeptierte Command mit Runtime-ID, Revision, kanonischem Action-JSON
  und exakt committed Snapshot unveränderlich journalisieren,
- Spielerprofile, Sessionteilnehmer, Spiele, Würfe, Gewinner und Endstände in
  derselben Transaktion als abfragbare Historienprojektion fortschreiben;
  Undo behält das Auditereignis, entfernt aber dessen Wertung,
- nach Prozessneustart ausschließlich den letzten Commit wiederherstellen,
- neue `runtime_instance_id` bei jedem Prozessstart,
- vollständige Snapshots per WebSocket publizieren.
- Projector-Companions per kurzlebigem Einmalcode koppeln, Grants ausschließlich
  als Hash persistieren, authentisierte Snapshots und Folgerevisionen streamen
  sowie aktive Verbindungen beim Widerruf schließen.

Noch offen und daher ausdrücklich kein Produktionsersatz:

- Teammodell sowie Heatmap-, Modusstatistik-, Export- und Trainingsabfragen,
- restliche Spielmodi und deklarative Effects,
- reale BlueZ-/Boardqualifizierung mit schneller Trefferfolge, Reconnect,
  Adapterausfall und Langzeittest,
- Migration vorhandener Python-Datenbanken,
- Umstellung der bestehenden UI auf API v2.

Wenn `SDB_ENABLE_BLE=1` gesetzt ist, meldet Health bis zum erfolgreichen
Gateway-Handshake `degraded` und Board `unavailable`. Erst nach Discovery,
Verbindung, Serviceprüfung und Notification-Subscription wechselt es zu
`ok`/`ready`. Bei deaktiviertem BLE bleibt Health `ok`/`disabled`.

## Datenbankschema

Schema 2 führt `runtime_journal` als append-only Auditspur ein. Schema 3 ergänzt
die mit der bisherigen Python-Datenbank kompatiblen Profil-, Session-, Spiel-,
Wurf- und Eventtabellen. Runtimezustand und diese fachliche Projektion werden
atomar geschrieben. Eine vorhandene Python-Schema-2-Datenbank wird nur ergänzt;
bestehende Profile und Historieneinträge bleiben erhalten.
Schema 4 ergänzt stabile Dart-Action-IDs in der schnellen Wurfprojektion.
Korrektur und Löschen markieren das ersetzte Event als unwirksam, hängen ein
neues Auditereignis an und schreiben alle betroffenen X01-Würfe aus dem
deterministisch wiedergegebenen Core-Zustand neu.
Schema 5 ergänzt widerrufbare Companion-Geräte. Gespeichert werden Geräte-ID,
Anzeigename, feste Projector-Rolle, Pairing- und Widerrufszeit sowie
ausschließlich der SHA-256-Token-Hash; ein Klartext-Token gelangt nie in SQLite.
Schema 6 ergänzt kleine, plattformübergreifende Hostpräferenzen. Schlüssel sind
streng begrenzt, Werte maximal 4 KiB groß; Spielzustand und Secrets gehören
ausdrücklich nicht in diese Tabelle.

Migrationen laufen fortlaufend und transaktional; eine Datenbank mit neuerer
unbekannter Schema-Version wird ohne Downgrade oder Schreibversuch abgelehnt. Nach jeder
Migration läuft `PRAGMA quick_check`. Da 1 → 2, 2 → 3, 3 → 4, 4 → 5 und 5 → 6
ausschließlich Tabellen beziehungsweise eine Spalte ergänzen, ist hierfür kein
destruktives Migrationsbackup erforderlich.
