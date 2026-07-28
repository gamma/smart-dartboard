# Technische Dokumentation: Eigenes Smart-Dartboard-System

Stand: 2026-07-28

## 1. Projektziel

Ziel ist die Entwicklung einer eigenen Computer-Anwendung, die ein elektronisches Bluetooth-Smart-Dartboard direkt anbindet, Treffer in Echtzeit ausliest und darauf aufbauend eigene Spiele, Trainingsmodi und Beamer-Visualisierungen ermöglicht.

Die Anwendung soll perspektivisch auf einem Computer laufen, der mit einem Beamer verbunden ist. Der Beamer projiziert visuelle Spielinhalte direkt auf oder um das Dartboard herum. Die physische Dartscheibe bleibt dabei die Eingabe-Hardware; die eigene Software übernimmt Spielregeln, Auswertung, Darstellung und Interaktion.

## 2. Kernziele

### 2.1 Kurzfristige Ziele

1. Dartboard per Bluetooth finden.
2. Verbindung zum Board herstellen.
3. Bluetooth-Services und Characteristics auslesen.
4. Notify-/Read-/Write-Kanäle identifizieren.
5. Rohdaten bei Treffern mitschneiden.
6. Bytefolgen konkreten Treffern zuordnen.
7. Einen ersten Treffer-Parser implementieren.
8. Treffer als strukturierte Events ausgeben.

Beispiel:

```json
{
  "type": "hit",
  "field": 20,
  "multiplier": 3,
  "label": "T20",
  "score": 60,
  "timestamp": "2026-07-28T14:00:00.000Z"
}
```

### 2.2 Mittelfristige Ziele

1. Stabile Verbindung inklusive Wiederverbindung.
2. Eigene lokale Spiel-Engine.
3. Erste Spiele: 301/501, Cricket, Count-Up, Around the Clock.
4. Benutzeroberfläche für Beamer-Ausgabe.
5. Spieler-, Runden- und Wurfverwaltung.
6. Trefferhistorie und Debug-Ansicht.

### 2.3 Langfristige Ziele

1. Projektion direkt auf das Dartboard.
2. Kalibrierung der Beamer-Projektion auf die Dartboard-Geometrie.
3. Interaktive Trainingsspiele mit hervorgehobenen Zielsegmenten.
4. Animationen, Sound, Effekte und Spielmodi.
5. Optional Online-/Netzwerkspiel.
6. Optional Editor für eigene Spielmodi.

---

## 3. Bekannte Informationen zu SDBplay 2 und kompatiblen Boards

Die App **SDBplay 2** ist die offizielle bzw. verbreitete App für bestimmte Smart-Dartboards, darunter vermutlich:

- SDB C1
- Kings Dart / Dartworld C1
- Karella CB-Smart
- weitere Boards, die auf derselben SDB-Plattform basieren

Öffentliche App-Beschreibung:

- Verbindung per Bluetooth zu einem Softdart-Ziel.
- Spielmodi wie 01 Games, Cricket, Count Up, Bull Hunters.
- Trainings-, Party- und Online-Modi.
- Fast 30 Spielvarianten.

Gefundene technische Gerätedaten aus Handbuch/Produktseiten:

- Bluetooth-Version: **Bluetooth 4.0**
- Stromversorgung: 2x AA oder 5V/1A über USB
- LED-Status:
  - Rot blinkend: wartet auf Verbindung
  - Blau dauerhaft: verbunden
  - Rot dauerhaft: mit externem Gerät wie Gamebox verbunden
- Hinweis im Handbuch: Erst Dartboard einschalten, dann SDB-App öffnen; App findet und verbindet automatisch.

Wichtig: Bluetooth 4.0 spricht stark dafür, dass das Board **Bluetooth Low Energy, BLE**, verwendet. Das ist für unsere Implementierung gut, weil BLE-Services und Characteristics relativ systematisch analysiert werden können.

---

## 4. Bluetooth-Grundlagen für dieses Projekt

## 4.1 Bluetooth Classic vs. Bluetooth Low Energy

Es gibt zwei relevante Bluetooth-Welten:

### Bluetooth Classic

- Älterer Standard.
- Wird oft für Audio, serielle Verbindungen, Headsets etc. genutzt.
- Kommunikation häufig über Profile wie SPP, HID, A2DP.
- Für kleine Sensordaten heute weniger typisch.

### Bluetooth Low Energy, BLE

- Ab Bluetooth 4.0 verbreitet.
- Typisch für Sensoren, Wearables, Spielzeug, einfache Controller.
- Geräte stellen sogenannte GATT-Services bereit.
- Jeder Service enthält Characteristics.
- Characteristics können gelesen, beschrieben oder abonniert werden.
- Trefferdaten werden sehr wahrscheinlich per Notify-Characteristic gesendet.

Da das Dartboard laut Handbuch Bluetooth 4.0 nutzt und batteriebetrieben ist, ist BLE die wahrscheinlichste Variante.

## 4.2 BLE-Begriffe

### Peripheral

Das Dartboard ist wahrscheinlich das BLE-Peripheral. Es sendet Advertisements und wartet auf eine Verbindung.

### Central

Unsere Anwendung ist der BLE-Central. Sie scannt, verbindet sich und liest bzw. abonniert Daten.

### Advertisement

Kurze Pakete, die das Board aussendet, solange es nicht verbunden ist. Daraus lassen sich oft erkennen:

- Gerätename
- Service-UUIDs
- Herstellerdaten
- Signalstärke RSSI

### GATT

GATT steht für Generic Attribute Profile. Darüber sind BLE-Daten organisiert.

### Service

Ein logischer Funktionsblock, identifiziert durch UUID.

Beispiele:

- Battery Service
- Device Information Service
- proprietärer SDB-Service

### Characteristic

Ein Datenpunkt innerhalb eines Service. Eine Characteristic kann z. B. Eigenschaften haben:

- Read: Wert kann gelesen werden
- Write: Wert kann geschrieben werden
- Write Without Response: schnelles Schreiben ohne Bestätigung
- Notify: Gerät sendet automatisch Daten an App
- Indicate: ähnlich Notify, aber mit Bestätigung

Für das Dartboard erwarten wir mindestens eine proprietäre Characteristic mit Notify für Trefferdaten.

---

## 5. Erwartete Bluetooth-Kommunikation

Wahrscheinlicher Ablauf:

1. Dartboard wird eingeschaltet.
2. Board blinkt rot und bewirbt sich per BLE Advertisement.
3. App scannt nach passendem Namen/Service.
4. App verbindet sich.
5. App entdeckt Services und Characteristics.
6. App aktiviert Notifications auf einer Characteristic.
7. Optional sendet die App Initialisierungsbefehle an eine Write-Characteristic.
8. Board sendet Trefferereignisse als Bytepakete.
9. App interpretiert Bytepakete in konkrete Felder.

Mögliche Rohdatenstruktur, rein hypothetisch:

```text
[Header] [Eventtyp] [Segment] [Multiplier] [Checksum]
```

Beispiel hypothetisch:

```text
A1 01 14 03 B9  -> Triple 20
A1 01 19 02 C0  -> Double 25 / Bull-Variante
A1 02 00 00 A3  -> Miss
```

Wichtig: Diese Bytefolgen sind noch nicht bekannt. Sie müssen empirisch ermittelt werden.

---

## 6. Öffentliche Protokollinformationen

Aktueller Recherche-Stand:

- Eine öffentliche, offizielle Protokolldokumentation für SDBplay 2 bzw. SDB C1 wurde bisher nicht gefunden.
- Öffentlich auffindbar sind App-Beschreibungen, Handbücher und Produktseiten.
- Das Handbuch bestätigt Bluetooth 4.0 und automatische Verbindung über SDB-App.
- Es wurden bisher keine öffentlich dokumentierten BLE-UUIDs, Characteristic-Tabellen oder Treffer-Bytecodes gefunden.
- Es gibt Hinweise auf die Android-App `com.sdbplay2.sdb`; die ältere/abweichende Paket-ID `com.sdbplay2` scheint nicht mehr korrekt zu sein.

Daraus folgt: Das Protokoll muss wahrscheinlich selbst analysiert werden.

---

## 7. Reverse-Engineering-Strategie

## 7.1 Phase 1: Passives Identifizieren

Ziel: Herausfinden, wie das Board im Bluetooth-Umfeld erscheint.

Benötigte Daten:

- Gerätename
- MAC-Adresse bzw. BLE-Adresse
- Advertisement-Daten
- Service-UUIDs
- Characteristic-UUIDs
- Characteristic-Properties

Geeignete Tools:

- Smartphone-App: nRF Connect
- Smartphone-App: LightBlue
- Linux: `bluetoothctl`
- Linux: `btmon`
- Python: `bleak`
- macOS: Bluetooth Explorer / PacketLogger, falls verfügbar

## 7.2 Phase 2: GATT-Analyse

Ziel: Services und Characteristics dokumentieren.

Zu erfassen:

```text
Service UUID
  Characteristic UUID
    Properties: read/write/notify/indicate
    Initial value, falls lesbar
```

Besonders wichtig:

- Welche Characteristic bietet Notify?
- Welche Characteristic erlaubt Write?
- Ändern sich Werte bei Treffern?
- Muss vor Notify ein Startbefehl geschrieben werden?

## 7.3 Phase 3: Treffer-Mapping

Ziel: Für jedes Dartsegment die Rohdaten erfassen.

Vorgehen:

1. Verbindung herstellen.
2. Notifications aktivieren.
3. Ein definiertes Segment drücken/werfen.
4. Rohpaket speichern.
5. Gleichen Treffer mehrfach wiederholen.
6. Tabelle aufbauen.

Beispieltabelle:

```text
Aktion       Rohdaten hex              Bemerkung
S1           unbekannt                 mehrfach erfassen
D1           unbekannt                 mehrfach erfassen
T1           unbekannt                 mehrfach erfassen
S20          unbekannt                 mehrfach erfassen
D20          unbekannt                 mehrfach erfassen
T20          unbekannt                 mehrfach erfassen
Bull 25      unbekannt                 mehrfach erfassen
Bull 50      unbekannt                 mehrfach erfassen
Miss         unbekannt                 falls Board Miss erkennt
```

## 7.4 Phase 4: Initialisierung verstehen

Falls das Board nach reiner Verbindung keine Daten sendet, macht SDBplay 2 wahrscheinlich eine Initialisierung.

Mögliche Initialisierungsarten:

- Write auf Control-Characteristic
- Login/Handshake
- Spielmodus setzen
- Boardtyp auswählen
- Heartbeat starten
- Zeit-/Session-Parameter übertragen

Dann müssen wir herausfinden, welche Writes die Original-App ausführt.

Mögliche Wege:

1. Android Bluetooth HCI Snoop Log aktivieren.
2. SDBplay 2 verbinden lassen.
3. Log exportieren.
4. Mit Wireshark auswerten.
5. GATT Writes/Notifications extrahieren.

## 7.5 Phase 5: Eigener Client

Wenn Services, Characteristics und Bytecodes bekannt sind:

- BLE-Verbindungsmodul implementieren.
- Notify abonnieren.
- Rohdaten parsern.
- Treffer-Events ausgeben.
- Wiederverbindung und Fehlerbehandlung einbauen.

---

## 8. Technische Anforderungen an die Bluetooth-Anbindung

### 8.1 Funktionale Anforderungen

Die Bluetooth-Schicht muss:

1. Nach Dartboards scannen.
2. Bekannte Boards wiedererkennen.
3. Verbindung herstellen.
4. Services und Characteristics auslesen.
5. Notifications abonnieren.
6. Optional Initialisierungsbefehle senden.
7. Rohdaten empfangen.
8. Trefferdaten interpretieren.
9. Verbindungsabbrüche erkennen.
10. Automatisch neu verbinden.
11. Debug-Logs schreiben.
12. Mehrere Board-Firmwarestände tolerieren.

### 8.2 Nichtfunktionale Anforderungen

1. Niedrige Latenz, ideal unter 100 ms nach Board-Meldung.
2. Robuste Verbindung über längere Spielzeit.
3. Saubere Fehlerzustände.
4. Keine App-Blockaden bei Verbindungsproblemen.
5. Gute Diagnoseausgaben.
6. Plattformstrategie klar definieren.

### 8.3 Fehlerfälle

Zu berücksichtigen:

- Board nicht eingeschaltet.
- Board bereits mit anderem Gerät verbunden.
- Bluetooth am Computer aus.
- Falsches Board ausgewählt.
- Notify-Characteristic nicht gefunden.
- Initialisierung schlägt fehl.
- Verbindung bricht während Spiel ab.
- Paketformat unbekannt oder verändert.
- Dart bleibt stecken und Segment sendet mehrfach oder gar nicht.
- Miss-Erkennung liefert Sondercode.

---

## 9. Plattform- und Technologieoptionen

## 9.1 Python + bleak

Sehr gut für Prototyping.

Vorteile:

- Schnelle Entwicklung.
- Plattformübergreifend: Windows, macOS, Linux.
- Gute BLE-Unterstützung.
- Ideal für Scanner, Logger und Parser.

Nachteile:

- GUI/Beamer-App später eventuell separat bauen.
- BLE-Verhalten unterscheidet sich je nach Betriebssystem.

Empfehlung: Für den ersten Protokoll-Prototyp sehr geeignet.

## 9.2 Node.js / Electron

Geeignet für spätere Desktop-App mit moderner UI.

Vorteile:

- Gute UI mit HTML/CSS/Canvas/WebGL.
- Beamer-Ausgabe einfach als Vollbildfenster.
- Spiele und Animationen gut umsetzbar.

Nachteile:

- BLE unter Node/Electron kann je nach Plattform aufwendiger sein.
- Native Module können Installation erschweren.

## 9.3 Web Bluetooth

Interessant für Browser-basierte Lösung.

Vorteile:

- Kein klassisches Installationspaket nötig.
- Gute Visualisierung im Browser.

Nachteile:

- Nicht auf allen Browsern/Plattformen verfügbar.
- iOS/Safari stark eingeschränkt.
- Pairing/Permissions können stören.

## 9.4 C#/.NET oder native App

Geeignet für eine stabile Windows-Anwendung.

Vorteile:

- Gute Windows-Integration.
- Stabile Vollbild-/Beamer-Ausgabe.

Nachteile:

- BLE-Code plattformspezifischer.
- Prototyping langsamer als Python.

## Empfehlung

1. Protokollanalyse mit Python + bleak.
2. Spiel- und Projektionsebene später entweder:
   - Python + Pygame/Qt,
   - Electron,
   - oder Web-App mit lokalem Python-BLE-Backend.

Eine gute Architektur wäre:

```text
Bluetooth Backend, Python
  -> WebSocket/Event API
Frontend/Beamer UI, Browser oder Electron
  -> Spiele, Animationen, Projektion
```

---

## 10. Software-Architektur

```text
+-----------------------------+
| Dartboard                   |
| BLE Peripheral              |
+-------------+---------------+
              |
              | Bluetooth LE
              v
+-----------------------------+
| Bluetooth Adapter           |
| Computer                    |
+-------------+---------------+
              |
              v
+-----------------------------+
| BLE Connector               |
| Scan, Connect, Reconnect    |
+-------------+---------------+
              |
              v
+-----------------------------+
| Protocol Layer              |
| Raw Bytes -> Dart Events    |
+-------------+---------------+
              |
              v
+-----------------------------+
| Event Bus                   |
| hit, miss, button, state    |
+-------------+---------------+
              |
              v
+-----------------------------+
| Game Engine                 |
| Rules, Turns, Scores        |
+-------------+---------------+
              |
              v
+-----------------------------+
| Projection UI               |
| Beamer, Animation, Audio    |
+-----------------------------+
```

## 10.1 Module

### BLE Connector

Verantwortlich für:

- Scannen
- Verbinden
- Service Discovery
- Notify abonnieren
- Reconnect
- Rohdaten-Logging

### Protocol Layer

Verantwortlich für:

- Paketformat erkennen
- Treffer dekodieren
- Sonderzustände erkennen
- Checksummen prüfen, falls vorhanden
- Events normalisieren

### Game Engine

Verantwortlich für:

- Spieler
- Runden
- Würfe
- Punkte
- Spielregeln
- Validierung
- Siegbedingungen

### Projection UI

Verantwortlich für:

- Vollbildausgabe
- Beamer-Auflösung
- Segment-Overlay
- Animationen
- Kalibrierung
- Sounds

---

## 11. Protokoll-Dokumentationsschema

Sobald reale Daten vorliegen, wird folgende Struktur gepflegt:

```markdown
## Board-Identifikation

- Modell:
- Firmware:
- Bluetooth-Name:
- BLE-Adresse:
- Advertisement-Daten:

## Services

| Service UUID | Bedeutung | Quelle |
|---|---|---|

## Characteristics

| Service UUID | Characteristic UUID | Properties | Bedeutung |
|---|---|---|---|

## Initialisierung

| Schritt | Richtung | UUID | Daten hex | Bedeutung |
|---|---|---|---|---|

## Trefferpakete

| Treffer | Rohdaten hex | Wiederholungen | Bedeutung |
|---|---|---|---|

## Sonderereignisse

| Ereignis | Rohdaten hex | Bedeutung |
|---|---|---|
```

---

## 12. Erste Testprogramme

## 12.1 BLE-Scanner

Aufgabe:

- Alle BLE-Geräte anzeigen.
- Name, Adresse, RSSI und Advertisement-Daten loggen.

## 12.2 GATT-Dumper

Aufgabe:

- Verbindung zum Board herstellen.
- Services und Characteristics ausgeben.
- Properties dokumentieren.

## 12.3 Notify-Logger

Aufgabe:

- Alle Notify-Characteristics abonnieren.
- Eingehende Daten mit Zeitstempel als Hex loggen.

## 12.4 Treffer-Mapping-Tool

Aufgabe:

- Nutzer gibt erwarteten Treffer ein, z. B. `T20`.
- Tool wartet auf nächstes Rohpaket.
- Tool speichert Zuordnung.

Beispiel:

```text
Bitte Treffer auslösen für: T20
Empfangen: A1 01 14 03 B9
Gespeichert: T20 -> A1 01 14 03 B9
```

---

## 13. Risiken und offene Punkte

### 13.1 Technische Risiken

- Protokoll ist proprietär und nicht dokumentiert.
- Board benötigt Initialisierung durch Original-App.
- Board sendet Daten nur in bestimmten Spielmodi.
- Trefferpakete enthalten Checksummen oder Verschlüsselung.
- Board akzeptiert nur eine Verbindung gleichzeitig.
- Unterschiedliche Boardmodelle nutzen unterschiedliche UUIDs.
- Betriebssysteme verhalten sich bei BLE unterschiedlich.

### 13.2 Rechtliche/ethische Hinweise

Das Ziel ist Interoperabilität mit eigener Hardware. Es sollte vermieden werden:

- Kopieren von App-Code.
- Umgehen von Kopierschutz.
- Nutzung fremder Server/APIs ohne Erlaubnis.
- Nachbau geschützter Online-Dienste.

Das reine Analysieren der eigenen Bluetooth-Hardware zur lokalen Nutzung ist typischerweise der sinnvollste und sauberste Weg.

---

## 14. Machbarkeitseinschätzung

Die Grundidee ist technisch gut machbar, wenn die Bluetooth-Daten des Boards dekodiert werden können.

Die Wahrscheinlichkeit ist gut, weil:

- Das Board laut Handbuch Bluetooth 4.0 nutzt.
- Die Original-App automatisch verbindet.
- Die Anwendung offenbar Treffer in Echtzeit erhält.
- Batteriebetriebene BLE-Peripherals meist einfache GATT-Kommunikation verwenden.

Die wichtigste unbekannte Variable ist das proprietäre Protokoll. Dieses muss praktisch getestet werden.

## 15. Nächster konkreter Meilenstein

Der nächste technische Meilenstein lautet:

> Eine kleine Testanwendung erstellt einen BLE-Scan, erkennt das Dartboard, verbindet sich und schreibt alle Services/Characteristics sowie eingehende Notify-Daten in ein Logfile.

Erfolgskriterium:

```text
Board gefunden
Verbindung hergestellt
Services gelistet
Notify-Daten bei Treffer empfangen
```

Wenn dieses Ziel erreicht ist, kann das Trefferprotokoll systematisch entschlüsselt werden.
