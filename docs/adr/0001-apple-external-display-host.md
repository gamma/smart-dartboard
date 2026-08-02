# ADR 0001: Nativer Apple-DisplayHost für Tauri

Stand: 2026-08-02

Status: **Implementiert; Hardwarequalifizierung ausstehend**

## Kontext

Die normative Zielarchitektur verlangt auf iPhone und iPad eine interaktive
Control-WebView und einen davon unabhängigen, nichtinteraktiven Projector über
AirPlay oder HDMI. Beide Views müssen dieselbe autoritative Rust-Runtime
verwenden.

Tauri 2.11.5 und Tao 0.35.3 können zusätzliche normale iOS-Scenes anfordern.
Eine vom System verbundene External-Display-Scene wird in der vorhandenen
Tauri-Hülle aber nicht als eigene Projector-WebView übernommen. Tao erzeugt bei
der Scene-Konfiguration außerdem grundsätzlich die Application-Rolle. Der ab
iOS/iPadOS 27 erforderliche External-Display-Scene-Accessory-Pfad ist in dieser
Version ebenfalls nicht vorhanden.

## Entscheidung

M0 verwendet für iOS/iPadOS bis einschließlich Version 26 einen dünnen nativen
`DisplayHost` in Objective-C++:

- `UIScreenDidConnectNotification` und `UIScreenDidDisconnectNotification`
  verwalten angeschlossene AirPlay-/HDMI-Ausgaben.
- Jedes externe Display erhält ein eigenes `UIWindow` mit nichtinteraktiver
  `WKWebView` und der Rolle `projector`.
- Die Rust-Runtime liefert Runtime-v2-Public-Snapshots und read-only Queries.
  Die WKWebView lädt `projector.html` und alle Artworks über den eingebetteten
  Tauri-Asset-Resolver; der Adapter enthält keine Regeln und keine zweite
  Runtime.
- Der Rückkanal meldet die Zahl aktiver Displays sowie Projector-Geometrie und
  Soundstatus. Debug-Builds erlauben zusätzlich ausschließlich kanonische
  `projector_test`-Würfe; Release-Builds lehnen diesen Command ab.
- Disconnect beendet weder Runtime noch Spiel; Reconnect erhält den letzten
  Snapshot.

Die kleine C-ABI-Grenze ist im Rust-Modul isoliert. Unsicherer FFI-Code bleibt
lokal erlaubt, während für den restlichen Crate weiterhin `unsafe_code = deny`
gilt.

## Nachweis

Der iPad-Air-Simulator mit iOS 26.5 und seinem separaten `TVOut` wurde verwendet:

- Control und `TVOut` zeigen gleichzeitig die gemeinsame Control- und
  Projector-Produkt-UI aus demselben Bundle.
- Der Projector meldet seinen realen WebView-Viewport `720 × 448` über die
  eingeschränkte C-ABI zurück; Runtime-Journal und Snapshot enthalten den
  `report_projector_geometry`-Command.
- Control erkennt Connect und Disconnect sofort.
- Nach Disconnect bleibt die autoritative Revision auf dem Controller erhalten.
- Nach Reconnect startet eine neue Projector-WKWebView mit dem letzten Snapshot.
- Der vollständige unsigned `aarch64-sim`-App-Build ist erfolgreich.

Der Debug-Schalter `--m0-test-hit-after-start` löst dafür einmalig denselben
Rust-Dispatchpfad wie der UI-Button aus. Er ist in Release-Builds wirkungslos.

## Folgen und offene Gates

- Der Simulator beweist Lifecycle, getrennte Framebuffer und Zustandsverteilung,
  aber keine reale AirPlay-/HDMI-Hardware oder Audioausgabe.
- Vor einem iOS-Produktrelease sind Tests mit mindestens einem qualifizierten
  iPhone, iPad, AirPlay-Empfänger und HDMI-/USB-C-Adapter Pflicht.
- Für iOS/iPadOS 27 wird ein Scene-Accessory-Adapter benötigt. Unterstützt Tauri
  ihn bis dahin zuverlässig, ersetzt er den M0-Adapter. Andernfalls bleibt der
  Rust-Core bestehen und Apple erhält eine dünne UIKit-Hülle.
- Der Companion-Modus ist von dieser Entscheidung unabhängig und benötigt
  weiterhin Bonjour, Pairing und einen authentisierten Netzwerktransport.

## Rückfallpfad

Schlägt die Hardwarequalifizierung fehl, bleibt Linux/Docker der erste
Produktionspfad. Auf Apple kann die vorhandene Rust-Runtime ohne Regel-Rewrite
in eine kleine Swift-/UIKit-Hülle eingebettet werden.
