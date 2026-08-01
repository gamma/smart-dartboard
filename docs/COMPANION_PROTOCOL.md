# Companion-Protokoll

Stand: 2026-08-01

Dieses Dokument konkretisiert den Companion-Modus aus der
[Cross-Platform-Architektur](CROSS_PLATFORM_ARCHITECTURE.md). Der erste
Produktfall ist ein iPhone oder iPad als Controller, BLE-Host, Runtime und
SQLite-Instanz sowie ein zweites iPad als reiner Projector mit Sound. Dieser
Modus ist der sekundäre Ausgabepfad nach der direkten Projector-Ausgabe desselben
Controller-Geräts über AirPlay oder HDMI. Beide Rollen werden von derselben App
bereitgestellt; der Companion ist keine zweite autoritative Spielanwendung.

## Autorität und Rollen

Nur das Controller-Gerät besitzt Boardverbindung, Runtime, Session und
Datenbank. Ein Companion erhält ausschließlich die Rolle `projector` und darf:

- einen vollständigen Runtime-Snapshot beziehen,
- fortlaufende Zustände und deklarative Projector-Effekte empfangen,
- seinen Verbindungs- und Audiozustand melden.

Er darf keine Spiel-, Setup-, Korrektur- oder BLE-Commands senden. Es gibt
keinen automatischen Hostwechsel während eines laufenden Spiels.

## Discovery, Pairing und Transport

Bonjour veröffentlicht nur Adresse, Port, Protokollversion und eine nicht
geheime Host-ID. Discovery ist keine Authentisierung. Der eigentliche Transport
muss gegenseitig gegen Downgrade geschützt und verschlüsselt sein; Klartext-
HTTP oder ein global geöffnetes CORS ist kein zulässiger Produktpfad.

Der Controller öffnet Pairing sichtbar im Board-Setup:

1. Er erzeugt einen sechsstelligen Einmalcode für fünf Minuten.
2. Das Companion-iPad wählt den gefundenen Controller. Der QR-Pfad überträgt
   zusätzlich Host-ID und SHA-256-Fingerprint der lokalen TLS-Identität. Beim
   manuellen Codepfad müssen beide Geräte vor dem Einlösen denselben kurzen
   Zertifikat-Fingerprint anzeigen und der Nutzer bestätigt die Übereinstimmung.
3. Nach höchstens fünf falschen Versuchen schließt das Fenster.
4. Bei Erfolg erhält das iPad einmalig einen zufälligen 256-Bit-Token mit der
   festen Rolle `projector`.
5. Der Controller speichert nur SHA-256 des Tokens; das iPad legt den Klartext
   im Keychain/Keystore ab.
6. Ein neues Pairing desselben Geräts ersetzt dessen alten Token. Der Betreiber
   kann jedes Gerät im Board-Setup widerrufen.

Diese Regeln liegen plattformneutral in `sdb-companion`. Bonjour, TLS,
QR-Erfassung und Keychain/Keystore sind austauschbare Hostadapter.

Der Token darf niemals über eine lediglich selbstsignierte, ungeprüfte
Verbindung übertragen werden. Ein Fingerprint aus dem ungeschützten Bonjour-
TXT-Record genügt nicht, weil ein aktiver Angreifer Service und Fingerprint
gemeinsam ersetzen könnte. Auf Apple-Geräten liegt die persistente lokale
TLS-Identität im Keychain. Der QR-Code bindet diese Identität direkt; der
manuelle Fallback benötigt den sichtbaren Vergleich auf beiden Geräten. Damit
bleibt das Pairing ohne vorherige Installation einer privaten Root-CA
point-and-click-tauglich.

## Replikation

Jeder Frame enthält:

- `protocol_version`,
- `runtime_instance_id`,
- `revision`,
- `kind` als `snapshot` oder `state`,
- den versionierten Payload.

Nach Connect, App-Resume, Runtimewechsel oder Revisionslücke fordert der
Companion zwingend einen Vollsnapshot an. Erst danach akzeptiert er
`revision + 1`. Derselbe Frame darf als Duplikat ignoriert werden; ältere
Frames werden verworfen. Ein `state` einer neuen Runtime darf niemals ohne
vorherigen Snapshot angewendet werden.

Die lokale Projector-WebView besitzt keine eigene Spielzustandskopie. Der
Companion-Adapter validiert den Frame und übergibt den freigegebenen Snapshot an
denselben read-only Projector-Renderer wie AirPlay/HDMI.

## Verhalten bei Ausfall

- Der Controller spielt bei Companion-Verlust weiter.
- Der Controller zeigt den Verlust und bietet lokale Vorschau oder
  AirPlay/HDMI als Fallback an.
- Der Companion zeigt einen neutralen Reconnect-Zustand, keinen veralteten
  vermeintlich aktuellen Spielstand.
- Nach Reconnect folgt zuerst ein Vollsnapshot; Effekte vor diesem Snapshot
  werden nicht nachträglich abgespielt.
- Sound wird über Effect-ID dedupliziert und bleibt standardmäßig der
  Projector-Rolle zugeordnet.

## Implementierungs- und Abnahmestand

Implementiert und automatisiert getestet:

- Einmalcode, Ablaufzeit und Versuchslimit,
- 256-Bit-Token, Hashspeicherung, Projector-Rolle und Widerruf,
- Snapshotpflicht, Runtimewechsel, Duplikate, Stale Frames und Revisionslücken,
- persistente, widerrufbare Geräte in SQLite-Schema 5; die Datenbank erhält nur
  den Token-Hash und behält den Widerrufszeitpunkt als lokale Auditspur,
- Headless-Pairing-API sowie Bearer-authentisierte Bootstrap- und
  WebSocket-Endpunkte. Jeder Socket beginnt mit einem Vollsnapshot, schließt bei
  einer Broadcast-Lücke und wird durch einen Gerätewiderruf aktiv beendet,
- natives Controller-Setup zum Öffnen des Pairing-Fensters, Anzeigen des
  ablaufenden Codes sowie Auflisten und Widerrufen persistierter Geräte. Diese
  Kommandos sind nur für die Control-WebView freigegeben; die Projector-WebView
  besitzt keine Companion-Verwaltungsrechte.
- Headless-Default-Deny: Ohne explizite Companion-Konfiguration antworten alle
  Pairing-, Geräte- und Projector-Routen mit `forbidden`. Bei externer
  TLS-Terminierung muss der Upstream ausschließlich auf Loopback lauschen; der
  kanonische Leaf-Fingerprint wird im Pairing-Bootstrap mitgeliefert.

Noch offen:

- Tokenablage des Companion-Clients im Apple Keychain beziehungsweise Android
  Keystore; die persistente lokale Apple-TLS-Identität des Controller-Hosts
  liegt bereits im Keychain,
- Bonjour-Advertiser und -Browser,
- native TLS-Terminierung beziehungsweise abgesicherte HTTPS/WSS-Auslieferung;
  der Rust-WebSocket ist implementiert, Klartext-LAN bleibt gesperrt,
- Rollenwahl und Pairing-UI auf Controller und Companion,
- echte iPhone/iPad-zu-iPad-Abnahme mit Disconnect, Resume und Widerruf.
