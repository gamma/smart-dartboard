# CI, Releases und Deployment

## GitHub-Automatisierung

Das Repository verwendet vier Workflows:

| Workflow | Auslöser | Aufgabe |
|---|---|---|
| `ci.yml` | Pull Requests, Push auf `main`, manuell | Python-Tests, Python-Kompilierung, JavaScript-Syntax, Compose-Prüfung, Website-Build, Container-Build und HTTP-Smoke-Test |
| `pages.yml` | relevante Pushes auf `main`, manuell | statische Website validieren und nach GitHub Pages deployen |
| `security.yml` | Pull Requests, Push auf `main`, montags, manuell | Dependency Review und CodeQL für Python und JavaScript |
| `container-release.yml` | veröffentlichtes GitHub Release | AMD64-/ARM64-Image nach GitHub Container Registry veröffentlichen, SBOM und Build-Provenienz erzeugen |

Dependabot prüft wöchentlich Python-Abhängigkeiten, GitHub Actions und das
Docker-Basisimage.

Für `main` sollten in **Settings → Branches → Branch protection rules**
mindestens diese Statuschecks verpflichtend sein:

- `Tests and static checks`
- `Container build and smoke test`
- `CodeQL (python)`
- `CodeQL (javascript-typescript)`

Direkte Pushes auf `main` können zusätzlich gesperrt werden. Für ein kleines
Projekt ist alternativ weiterhin ein direkter Push möglich; die Workflows
melden dann Probleme, verhindern den Push aber nicht rückwirkend.

Der Apple-Job erzeugt außerdem ein unsigniertes `Smart Dartboard.app`, packt
es mit `ditto` als ressourcen- und rechteerhaltendes ZIP und hängt es für sieben
Tage als Workflow-Artefakt an. Dieses Artefakt dient ausschließlich Build- und
Starttests. Für eine Verteilung außerhalb des Entwicklungs-Macs fehlen noch
Developer-ID-Signatur, Apple-Notarisierung und Stapling; dafür dürfen
Zertifikate und Notarisierungszugänge nur als geschützte Repository-Secrets in
einem separaten Release-Workflow hinterlegt werden.

## GitHub Pages

Die Projektwebsite wird bei relevanten Änderungen auf `main` automatisch
veröffentlicht. Ein manueller Lauf bleibt möglich:

```bash
gh workflow run pages.yml
gh run list --workflow pages.yml --limit 1
gh run watch <RUN_ID> --exit-status
```

Unter **Settings → Pages** muss als Quelle **GitHub Actions** gewählt sein.
Zusätzlich sollte **Enforce HTTPS** aktiviert werden.

## Container veröffentlichen

Ein veröffentlichtes GitHub Release baut automatisch:

```text
ghcr.io/gamma/smart-dartboard:<version>
ghcr.io/gamma/smart-dartboard:<major>.<minor>
ghcr.io/gamma/smart-dartboard:sha-<commit>
ghcr.io/gamma/smart-dartboard:latest
```

`latest` wird nur für ein reguläres Release gesetzt, nicht für ein
Pre-Release. Das aktuelle Release wird so angelegt:

```bash
gh release create v0.0.2 --generate-notes --title "Smart Dartboard 0.0.2"
```

Der Release-Workflow veröffentlicht ein Multi-Arch-Image für
`linux/amd64` und `linux/arm64`. Damit läuft dasselbe Release auf einem
üblichen Mini-PC und auf einem 64-Bit-Raspberry-Pi.

Der Rust-Migrationspfad besitzt zusätzlich ein minimales Linux-BLE-Sidecar.
Alle 24 Spielmodi und Oberflächen sind inzwischen portiert; bis zur realen
BlueZ-/Board- und Bedienabnahme wird der Pfad trotzdem nur lokal aus
`Dockerfile.rust` und `Dockerfile.ble` gebaut und nicht als produktives
Release-Image beworben. Container-Build, authentisierter Rohpaket-Ingress und
der verlustfreie Import einer bestehenden `data/dartboard.db` sind belegt. Die
kontrollierte Import- und Rollback-Anleitung steht in
[Rust Headless Server API v2](RUST_SERVER_V2.md#einmaliger-import-der-python-datenbank).

Nach dem ersten Release die Paketseite unter GitHub öffnen und die Sichtbarkeit
des Containerpakets prüfen. Für ein öffentliches Repository sollte das Image
öffentlich lesbar sein. Bei einem privaten Paket benötigt das Zielgerät einen
GitHub-Token mit `read:packages`.

## Erstinstallation aus einem Release

Auf dem Linux-Zielgerät:

```bash
git clone https://github.com/gamma/smart-dartboard.git
cd smart-dartboard
cp .env.example .env
```

In `.env` mindestens das veröffentlichte Release eintragen:

```dotenv
SDB_VERSION=0.0.2
SDB_DEVICE_NAME=SDB-BT
SDB_DEVICE_ADDRESS=
```

Anschließend:

```bash
mkdir -p data
docker compose -f compose.production.yml pull
docker compose -f compose.production.yml up -d
docker compose -f compose.production.yml ps
curl http://localhost:8000/api/health
```

Während der Ersteinrichtung darf die Adresse leer bleiben. Für den dauerhaften
Spielhallenbetrieb sollte `SDB_DEVICE_ADDRESS` auf die feste BLE-Adresse des
Boards gesetzt werden.

## Kontrolliertes Update

Zuerst `SDB_VERSION` in `.env` auf die neue Version setzen. Das neue Image kann
dann vor der Unterbrechung geladen werden, während der bisherige Container
weiterläuft:

```bash
docker compose -f compose.production.yml pull
```

Für ein konsistentes SQLite-Backup danach kurz stoppen:

```bash
docker compose -f compose.production.yml stop
cp data/dartboard.db "data/dartboard-$(date +%Y%m%d-%H%M%S).db"
```

Nun das bereits geladene Release starten:

```bash
docker compose -f compose.production.yml up -d
docker compose -f compose.production.yml ps
curl http://localhost:8000/api/health
```

Erst nach erfolgreichem Healthcheck Controller und Projektor wieder für den
Spielbetrieb freigeben.

## Rollback

Bei einem fehlgeschlagenen Update:

1. `SDB_VERSION` in `.env` auf die vorherige Version zurücksetzen.
2. Den vorherigen Datenbankstand nur dann wiederherstellen, wenn das neue
   Release Daten verändert hat, die mit der alten Version nicht funktionieren.
3. Das alte Image starten und den Healthcheck prüfen.

```bash
docker compose -f compose.production.yml up -d
curl http://localhost:8000/api/health
docker compose -f compose.production.yml logs --tail=100 dartboard
```

Ein automatisches Deployment direkt aus GitHub Actions auf den lokalen
Spielhallenrechner ist bewusst nicht eingerichtet. Das Zielsystem bleibt im
isolierten Netz; Version, Zeitpunkt und Rollback bleiben unter Kontrolle des
Betreibers.
