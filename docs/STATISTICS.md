# Statistik, Telemetrie und Replay

Stand: 2026-07-30

## Ziel

Smart Dartboard speichert gewertete Sessions und Spiele lokal, damit die
Control-UI Session- und Langzeitstatistiken, Treffer-Heatmaps, Replays und
datenbasierte Trainingshinweise anzeigen kann. Die Daten dienen außerdem dazu,
Schwierigkeit und Abbruchquote einzelner Spielvarianten zu beurteilen.

Die Statistikansicht ist auf der Startseite, in der Spielauswahl und in der
Session-Zusammenfassung über **Statistiken** erreichbar.

## Was gespeichert wird

### Session

- Sprache (`de` oder `en`)
- Spieler und Reihenfolge
- Start, Ende und Status
- enthaltene Spiele

### Spiel

- Spielmodus und vollständige Optionen
- Ruleset-, App- und Umgebungsversion
- Spielerreihenfolge und Endstände
- Ergebnisart, Gewinner, Abschlussgrund und Zeitstempel
- initialer und finaler Replay-Zustand

### Wurf

- unverändertes Eingabeereignis
- Feld, Ring, Multiplikator und klassischer Dartwert
- modusspezifische Punkteänderung und Ergebnisart
- Runde, Dart innerhalb der Aufnahme, Spieler und Quelle
- die vor dem Wurf sichtbare Aufgabe mit Ziel- und Gefahrenfeldern
- der nach dem Ereignis sichtbare Replay-Frame

`game_events` ist das unveränderliche Audit-Journal. Undo und Korrekturen
löschen dort keine Vergangenheit, sondern ergänzen ein Korrekturereignis und
markieren das ersetzte Wurfereignis als nicht mehr wirksam. Die Tabelle
`throws` enthält parallel den aktuellen, korrigierten Wurfstand für schnelle
Auswertungen.

## Wertung und Datenqualität

- Nur Spiele mit Status `finished` fließen in Langzeitstatistiken und Heatmaps
  ein.
- Abgebrochene und nach einem Neustart verwaiste Spiele bleiben als Auditdaten
  erhalten, zählen aber nicht.
- Ein per Projektor angeklickter Testwurf markiert das gesamte Spiel als
  `test`. Testspiele werden standardmäßig ausgeblendet und können in der
  Statistikansicht bewusst eingeblendet werden.
- Alte Datenbanken werden beim Start automatisch auf Schema-Version 2
  migriert. Alte Würfe bleiben nutzbar; Replays werden daraus bestmöglich
  rekonstruiert. Aufgaben- und Frame-Telemetrie existiert naturgemäß erst für
  neu erfasste Würfe.
- Modusstatistiken werden nach Spielmodus, Ruleset-Version und Optionssatz
  getrennt. Dadurch lassen sich zum Beispiel Easy und Hard vergleichen, ohne
  Ergebnisse verschiedener Regeln zu vermischen.

## Begriffe

Die UI verwendet die international üblichen Darts-Begriffe:

- `Single`, `Double`, `Triple`
- `Single Bull` / `SBull`
- `Double Bull` / `DBull`
- `Miss`, `Bust`, `Checkout`, `Double Out`
- `Dart` und `Visit` (deutsch: Aufnahme)

Alle übrigen UI-Texte werden durch das zentrale Wörterbuch in
`web/static/i18n.js` auf Deutsch oder Englisch ausgegeben. Die Sprache wird
vor dem Start einer Session gewählt und gilt auf Control und Projector.

## API

```text
GET /api/history/sessions
GET /api/history/sessions/{session_id}
GET /api/history/games/{game_id}
GET /api/history/games/{game_id}/replay
GET /api/statistics/players
GET /api/statistics/heatmap
GET /api/statistics/modes
GET /api/training/{player_id}/recommendations
GET /api/data/export
```

Der Rust-Pfad stellt alle Leseverträge parallel versioniert unter
`/api/v2/history/...`, `/api/v2/statistics/...`, `/api/v2/training/...` und
`/api/v2/data/export` bereit. Headless-Server und native Apps verwenden dasselbe
SQLite-Read-Model; die gemeinsame Produkt-UI normalisiert lediglich die
versionierten Antwortumschläge. Sessiondetail, Spieldetail und Replay enthalten
dort auch die unveränderliche Korrektur- und Löschkette.
Bei älteren, wurfbasierten Datensätzen ohne `game_events` rekonstruiert der
Rust-Adapter weiterhin einen bestmöglichen Replay-Frame je gespeichertem Wurf.

Die Statistikendpunkte akzeptieren bei Bedarf `include_test=true`. Die Heatmap
kann zusätzlich nach `player_id`, `session_id` und `game_type` gefiltert
werden.

Der Export behält für Kompatibilität das portable Archivformat
`schema_version: 2` und nennt die tatsächlich verwendete Rust-Datenbankversion
separat als `database_schema_version`. Runtime-Einstellungen, Boarddaten,
Companion-Tokens und andere Secrets werden nicht exportiert.

## Datenschutz und Aufbewahrung

Alle Daten liegen ausschließlich in der lokalen SQLite-Datei
`data/dartboard.db` beziehungsweise unter `SDB_DATA_DIR`. Es findet keine
Cloud-Synchronisierung statt.

Historische Daten werden absichtlich unbegrenzt aufbewahrt, bis der Betreiber
die lokale Datenbank löscht oder ersetzt. Vor Wartung oder Löschung kann die
vollständige Historie in der Statistikansicht als JSON exportiert werden. Das
Exportarchiv enthält Spieler- und Spieldaten, jedoch keine
Projektorkalibrierung, Hardwareadresse oder sonstigen Runtime-Einstellungen.

Da Spielernamen personenbezogen sein können, gehört der Server in ein
isoliertes Dartboard-Netz. Die History- und Export-API sollte nicht über das
Internet veröffentlicht werden.
