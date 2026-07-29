# Publishing

## GitHub Pages

Die statische Projektwebsite liegt unter `website/`. Der Build übernimmt die
bereits versionierten Modus-Artworks aus `web/static/assets/`, fügt die
WebKit-Gameplay-Aufnahmen hinzu und erzeugt `dist/`.

```bash
bash website/build.sh
python3 -m http.server 8080 --directory dist
```

Danach ist die lokale Vorschau unter `http://localhost:8080` erreichbar.
`dist/` ist ausschließlich ein generiertes Build-Verzeichnis und wird nicht
versioniert.

Für eine schnelle Vorschau direkt aus dem Quellbaum:

```bash
python3 -m http.server 8080
```

Dann `http://localhost:8080/website/` öffnen. Die auf der Landingpage
verwendeten Cover liegen zusätzlich unter `website/assets/`, damit diese
Vorschau keine fehlenden Bilder zeigt. Der Pages-Build übernimmt weiterhin die
kanonischen Originale aus `web/static/assets/`.

Der Workflow `.github/workflows/pages.yml` verwendet die offiziellen
GitHub-Pages-Actions und wird bewusst nur manuell über `workflow_dispatch`
ausgelöst. Vor dem ersten Lauf muss unter **Settings → Pages → Build and
deployment** die Quelle **GitHub Actions** gewählt werden.

## Gameplay-Aufnahmen erneuern

Die Screenshots entstehen aus einer frischen lokalen Session und nicht aus
statischen Mockups:

```bash
SDB_ENABLE_BLE=0 SDB_ALLOW_TEST_EVENTS=1 \
  SDB_DATA_DIR=/tmp/smart-dartboard-capture \
  .venv/bin/uvicorn app:app --host 127.0.0.1 --port 8777

node website/capture-screenshots.mjs
```

Das Capture-Skript verwendet standardmäßig das global installierte Playwright
mit WebKit. Ein abweichendes Modul oder eine andere laufende Instanz kann über
`PLAYWRIGHT_MODULE` beziehungsweise `SDB_CAPTURE_URL` gesetzt werden.
Projektor-Testwerkzeuge werden für die Marketing-Aufnahmen ausgeblendet, weil
sie im normalen BLE-Spielbetrieb ebenfalls nicht sichtbar sind.

## Freigabecheck

Vor einer öffentlichen Veröffentlichung:

1. `bash website/build.sh` erfolgreich ausführen.
2. Desktop und Mobile in WebKit visuell prüfen.
3. Alle Links, Alternativtexte und den Lightbox-Dialog testen.
4. `node --check website/app.js` ausführen.
5. Die Aussagen zu Moduszahl und Funktionen mit `README.md` abgleichen.
6. Den endgültigen Rechteinhaber in `NOTICE`, `ASSETS_LICENSE.md` und den
   Attributionstexten bestätigen.
7. Herkunft und Nutzungsrechte aller veröffentlichten Artworks klären und
   freigegebene Pfade in die Tabelle `Cleared assets` in
   `ASSETS_LICENSE.md` eintragen. Der aktuelle Altbestand ist in
   `docs/ARTWORK_PROMPTS.md` als unbekannt markiert.
8. Erst danach den manuellen Pages-Workflow starten.

Apache-2.0 ist für den Code eingerichtet. Die Punkte 6 und 7 bleiben vor einem
öffentlichen Release echte Blocker und sollten nicht durch Vermutungen ersetzt
werden.
