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

## Endpunkte

| Methode | Pfad | Zweck |
| --- | --- | --- |
| `GET` | `/api/v2/health` | Runtime-, Datenbank-, Board-, Protokoll- und Schemastatus |
| `GET` | `/api/v2/runtime/bootstrap` | vollständiger versionierter Snapshot |
| `GET` | `/api/v2/runtime/snapshot` | erneuter Snapshot nach Lücke oder Reconnect |
| `POST` | `/api/v2/runtime/commands` | ein `CommandEnvelope` atomar anwenden |
| `GET` | `/api/v2/runtime/events` | WebSocket mit initialem und folgenden Snapshots |
| `GET` | `/api/v2/players` | persistente Spielerprofile |
| `GET` | `/api/v2/history/sessions` | Sessionhistorie; optional `?limit=` |
| `GET` | `/api/v2/statistics/players` | Langzeitstatistik aus gewerteten Produktionsspielen |

Browser-POSTs und WebSockets akzeptieren nur dieselbe Origin. Clients ohne
`Origin`, etwa lokale Diagnosewerkzeuge, bleiben möglich. Unvereinbare
Protokollversionen, falsche Runtime-IDs und veraltete Revisionen liefern stabile
Fehlercodes und passende HTTP-Statuscodes.

## Aktueller Funktionsumfang

- CountUp und X01 starten,
- Session mit vollständigen Spielerreferenzen starten, Modus vorbereiten und
  Startspieler festlegen,
- Countdown, Spiel, Ergebnis, nächste Spielauswahl, Rematch und
  Sessionzusammenfassung als gemeinsamen Screenfluss führen,
- Einzel- und Koop-Siege mit drei Sessionpunkten je Gewinner werten;
  Unentschieden und Abbrüche bleiben punktlos,
- kanonische Dart-Events übernehmen,
- die Wurfquelle transportneutral als `board`, `projector_test` oder
  `manual_correction` führen; ein Projektor-Testwurf markiert das gesamte Spiel
  als Test und schließt es aus der Standardstatistik aus,
- Turn fortsetzen und Undo; ein Undo des Siegtreffers öffnet zugleich das Spiel
  wieder und nimmt die Sessionwertung atomar zurück,
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

Noch offen und daher ausdrücklich kein Produktionsersatz:

- Teammodell sowie vollständige History-, Replay-, Heatmap-, Modusstatistik-
  und Exportabfragen,
- restliche Spielmodi und deklarative Effects,
- Wurfkorrektur und Löschen über den öffentlichen Contract,
- Bleak-/BlueZ-Gateway und reale Boardqualifizierung,
- Migration vorhandener Python-Datenbanken,
- Umstellung der bestehenden UI auf API v2.

Wenn `SDB_ENABLE_BLE=1` gesetzt ist, meldet Health derzeit `degraded` und Board
`unavailable`. Das verhindert, dass ein Container ohne implementierten
Boardadapter fälschlich als produktionsbereit erscheint.

## Datenbankschema

Schema 2 führt `runtime_journal` als append-only Auditspur ein. Schema 3 ergänzt
die mit der bisherigen Python-Datenbank kompatiblen Profil-, Session-, Spiel-,
Wurf- und Eventtabellen. Runtimezustand und diese fachliche Projektion werden
atomar geschrieben. Eine vorhandene Python-Schema-2-Datenbank wird nur ergänzt;
bestehende Profile und Historieneinträge bleiben erhalten. Migrationen
laufen fortlaufend und transaktional; eine Datenbank mit neuerer unbekannter
Schema-Version wird ohne Downgrade oder Schreibversuch abgelehnt. Nach jeder
Migration läuft `PRAGMA quick_check`. Da 1 → 2 und 2 → 3 ausschließlich neue
Tabellen ergänzen, ist hierfür kein destruktives Migrationsbackup erforderlich.
