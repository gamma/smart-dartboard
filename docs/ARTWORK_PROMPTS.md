# Reproduzierbare Spielmodus-Artworks

Die Cover unter `web/static/assets/modes/<slug>.webp` verwenden eine gemeinsame
fröhliche 3D-Cartoon-Bildsprache. Die Scheibe bleibt als Dartboard klar
erkennbar, die Welt darum wirkt jedoch wie ein hochwertiges handgebautes
Arcade-Spielzeug: warm, farbenfroh, charmant und familienfreundlich.

## Referenzbilder

Neue Cover verwenden `web/static/assets/modes/heart_chase.webp` als
Stilreferenz. Motive und Requisiten kommen aus dem jeweiligen Modus-Prompt.

## Theme-Packs

Das aktive Artwork-Theme wird im Board-Setup gewählt und dauerhaft gespeichert:

- `cartoon` verwendet `web/static/assets/modes/<slug>.webp` und ist Standard.
- `neon` verwendet die erhaltenen Bestandscover unter
  `web/static/assets/themes/neon/modes/<slug>.webp`.

Das Classic-Neon-Pack enthält die 13 historischen Cover. Modi, die erst danach
entstanden sind, fallen im Neon-Theme automatisch auf ihr Cartoon-Cover zurück.
Dadurch bleibt das historische Pack unverändert erhalten, ohne für neue Modi
künstlich Neonbilder erzeugen zu müssen.

Der projektlokale Skill
`.agents/skills/smart-dartboard-artwork/SKILL.md` beschreibt den verbindlichen
Generierungs-, Prüf- und Publishing-Ablauf für beide Packs.

## Basis-Prompt

```text
Use case: stylized-concept
Asset type: cinematic game-mode cover for a touch-controlled smart dart arcade
Primary request: Create a joyful, playful 3D cartoon interpretation of the specified dart game mode.
Input images: the Heart Chase cover is a style reference only.
Scene/backdrop: a bright handcrafted miniature arcade world with painted wood, soft fabric, paper, clay and toy-like props; warm daylight or cheerful fairground lighting.
Subject: one recognizable regulation dartboard remains the hero object; surround it with a few large, readable mode-specific props and friendly visual storytelling.
Style/medium: polished stylized 3D animation render with rounded shapes, tactile materials, expressive charm and believable depth; cartoon-like but not flat vector art.
Composition/framing: landscape 3:2; dartboard centered slightly right; clear darker negative space on the left and lower-left for UI copy; strong depth and perspective; no important detail at the outer edges.
Lighting/mood: soft warm key light, gentle shadows, optimistic and inviting; crisp focal board with restrained depth of field.
Color palette: warm cream, sky blue, leafy green, coral, mustard and mode-specific colors; natural dartboard colors may be simplified but remain recognizable.
Constraints: one coherent scene; regulation board geometry; visually readable at card size; no people; no logos; no brands; no watermark; no frame; no UI.
Avoid: any text, letters or title typography, poster layout, flat vector art, duplicated boards, deformed darts, dark nightclub scenes, black void backgrounds, neon tubes, cyberpunk, holograms, lasers, heavy bloom, ominous mood, photoreal product advertising.
```

## Classic-Neon-Basis-Prompt

Neue Neon-Cover verwenden
`web/static/assets/themes/neon/modes/countup.webp` als primäre Stilreferenz.
Je nach Motiv dient zusätzlich eines der Bestandscover als Akzentreferenz:
`target_rush.webp` für Cyan-Ziellicht, `treasure_hunt.webp` für warmes Gold und
`boss_fight.webp` für violette Energie. Referenzen bestimmen ausschließlich
Stil und Bildaufbau; ihre konkreten Treffer, Pfeile und Effekte werden nicht
kopiert.

```text
Use case: stylized-concept
Asset type: cinematic game-mode cover for a touch-controlled smart dart arcade
Primary request: Create a premium cinematic Classic Neon interpretation of the specified dart game mode, matching the supplied historical Smart Dartboard covers.
Input images: the Countup cover is the primary style reference; the optional second cover is an accent and lighting reference only.
Scene/backdrop: a dark upscale arcade or tournament hall with restrained architectural detail, atmospheric depth and a subtly reflective floor or counter.
Subject: one recognizable regulation sisal dartboard is the hero object; express the game mode through a small number of readable darts, practical light accents and physically plausible themed props.
Style/medium: high-detail cinematic 3D render with realistic tactile board fibers, metal dividers, premium dark materials and controlled sci-fi arcade lighting; dramatic but believable rather than cartoon or graphic design.
Composition/framing: landscape 3:2; dartboard centered slightly right and shown large; generous dark negative space on the left and lower-left for UI copy; strong depth; no important detail at the outer edges.
Lighting/mood: low-key premium arcade lighting, crisp board face, cyan/blue foundation with one mode-specific accent color, selective rim light, restrained haze and bloom, readable shadow detail.
Color palette: black, charcoal and natural dartboard cream/green/red, anchored by electric cyan or blue plus one mode-specific accent such as amber, magenta, violet or green.
Constraints: one coherent scene; regulation board geometry and number order; visually readable at card size; no people; no logos; no brands; no watermark; no frame; no UI.
Avoid: any text, letters or title typography, poster layout, flat vector art, playful toy materials, cute mascots, duplicated boards, malformed darts, impossible board geometry, generic cyberpunk city scenery, excessive laser clutter, blown highlights, heavy bloom obscuring segments, black crush, horror imagery.
```

## Modusspezifische Ergänzungen

| Slug | Motiv und Akzent |
|---|---|
| `countup` | Koralle/Gold/Grün; aufsteigende Holzstufen, Pfeile und Sternmarken als freundliches Punkterennen |
| `x01` | Koralle/Blau/Grün; absteigende Spielsteine führen zu einer kleinen Zielfahne |
| `cricket` | Grasgrün/Senf; freundliche Filz-Grillen, kleine Holzschläger und sechs Zielmarken |
| `target_rush` | Himmelblau/Koralle; bewegte Zielscheiben auf einer hölzernen Rennbahn mit Zielfahne |
| `avoid_bomb` | Grün/Koralle; harmlose schwarze Spielzeugbomben zwischen sicheren grünen Zielchips, keine Explosion |
| `color_clash` | Koralle/Himmelblau; zwei freundliche Farbteams bemalen hölzerne Gebietsplättchen |
| `risk_it` | Rosa/Gold; Holzwaage mit Sparschwein und wachsendem Stapel aus Sternchips |
| `king_of_board` | Gold/Koralle; große gepolsterte Krone, kleine Bilderbuchburg und Turnierfähnchen |
| `treasure_hunt` | Gold/Grün; helle Spielzeuginsel mit Schatzkiste, Blankokarte, Kompass und Trittsteinen |
| `boss_fight` | Koralle/Himmelblau; großer freundlicher Filz-Monsterboss mit Spielzeugrüstung und kleinen Koop-Helden |
| `darts_bingo` | Creme/Gold/Grün; Raster aus abgerundeten Holzplättchen, die mit Sternen und Blättern gestempelt werden |
| `lightning_round` | Senfgelb/Koralle; springende weiche Blitz-Maskottchen, geschlossene Spielzeug-Stoppuhr und Bewegungsbögen |
| `simon_says` | Koralle/Grün/Blau/Gold; vier große Stofftaster und eine Spur farbiger Erinnerungssterne |
| `heart_chase` | Koralle/Gold; freundliche große Stoffherzen verfolgen sich wie Spielfiguren rund um die Scheibe |
| `robin_hood` | Waldgrün/Gold; Spielzeugpfeile, kleine Zielscheiben und freundliche Waldkulisse |
| `dragon_eggs` | Orange/Grün; bunte Eier, ein neugieriger kleiner Drache und warme Burg-Spielzeugwelt |
| `ghost_chase` | Mint/Creme; ein rundlicher freundlicher Geist saust mit Stoffschweif um das Board |
| `cookie_monster` | Keksbraun/Blau; fröhliche Kekse, Milchflasche und verspielte Küchenrequisiten |
| `space_defender` | Himmelblau/Grün/Gold; freundliche Spielzeugraumschiffe und kleine runde Planeten |
| `candy_cannon` | Koralle/Gelb; Bonbonkanone aus bemaltem Holz mit fliegenden Süßigkeiten |
| `mini_golf` | Grasgrün/Creme; Mini-Golf-Bahn, kleine Fahne und sanfte Hügel rund um das Board |
| `eight_ball` | Billardgrün/Gold; bunte Billardkugeln und kleine Holztisch-Elemente |
| `block_drop` | Koralle/Senf/Mint; bunte abgerundete Puzzleblöcke stapeln sich spielerisch |
| `dart_sweeper` | Salbeigrün/Orange; freundliche Spielzeugminen, Fragezeichen-Plättchen und aufgedeckte Zahlenchips |

## Ausgabe

Generierte Bilder werden mittig auf `900 × 640 px` beschnitten und als
verlustbehaftetes WebP mit Qualitätsstufe 88 gespeichert. In das Bild wird kein
Titel eingebrannt; Titel und Beschreibung kommen barrierefrei aus der Web-UI.

## Prompt- und Herkunftsnachweis

Für jedes neu erzeugte oder ersetzte Asset wird zusammen mit der Bildänderung
festgehalten:

- Theme, Slug und endgültiger vollständiger Prompt,
- verwendete Referenzdateien,
- Generator und Modellbezeichnung,
- Generierungsdatum,
- wesentliche Nachbearbeitung sowie
- bekannte Lizenz- oder Nutzungsbedingungen.

Fehlende Angaben historischer Bestandsbilder werden als `unbekannt` markiert
und nicht nachträglich geraten. Für ein Publishing-Review ist unbekannte
Herkunft oder ungeklärte Nutzung ausdrücklich als offener Punkt zu behandeln.

### Aktueller Altbestand

Nach Angabe des Projektinhabers wurden alle Bestandsgrafiken lokal mit OpenAI
ImageGen erzeugt. Der Generierungsablauf wurde überwiegend durch GPT-5.6-sol
gesteuert. Die lokale Codex-Sitzung und die noch vorhandenen
ImageGen-Originale erlauben inzwischen eine genauere Rekonstruktion:

- 50 direkte ImageGen-Aufrufe sowie ein anfänglicher Batch mit drei
  ImageGen-Aufrufen liefen in einer mit `gpt-5.6-sol` gesteuerten
  Codex-Sitzung.
- Die C2PA-Manifeste der geprüften Original-PNGs nennen
  `OpenAI Media Service API`, den Software-Agenten `gpt-image` in Version `2.0`
  und den digitalen Quelltyp `trainedAlgorithmicMedia`.
- Aus insgesamt 53 erzeugten Ergebnissen wurden die heute vorhandenen 37 Cover
  und 13 Effekte ausgewählt; drei Cover-Kandidaten wurden verworfen oder
  ersetzt. Sämtliche übergebenen Bildreferenzen zeigen auf bereits lokal
  erzeugte Dateien dieses Projekts. Fremde Bilder, Logos oder
  Markenreferenzen wurden in den protokollierten Aufrufen nicht verwendet.
- Die finalen WebP-Dateien enthalten das C2PA-Manifest nicht mehr, weil es bei
  der lokalen Konvertierung nicht übernommen wurde. Die Zuordnung bleibt über
  ImageGen-Call-ID, Konvertierungsbefehl und Git-Commit rekonstruierbar.

| Bestand | Erzeugung und Referenzen | Bildmodell/Datum | Nachbearbeitung | Lizenzstatus |
|---|---|---|---|---|
| Playful-Cartoon-Cover | 24 ImageGen-Ergebnisse; `heart_chase` nur aus Text erzeugt, danach als interne Stilreferenz für alle weiteren Cover | `gpt-image` 2.0; 29.07.2026; gesteuert mit `gpt-5.6-sol` | ImageMagick: mittiger Zuschnitt auf 900 × 640, WebP-Qualität 88 | CC BY-NC-SA 4.0; Gerry Weißbach (gamma / gamma production) |
| Classic-Neon-Cover | 13 ImageGen-Ergebnisse; `countup`, `x01` und `cricket` nur aus Text, danach als interne Stilreferenzen für die weiteren Neon-Cover | `gpt-image` 2.0; 28.–29.07.2026; gesteuert mit `gpt-5.6-sol` | historische Versionen aus Git-Commit `54ba4a3` wiederhergestellt, mittig auf 900 × 640 gebracht und als WebP mit Qualität 88 exportiert | CC BY-NC-SA 4.0; Gerry Weißbach (gamma / gamma production) |
| animierte 3D-Props | 13 ImageGen-Ergebnisse; zwölf Props mit `heart_chase` als interner Referenz, `candy_overheat` mit `candy_cannon` und `candy` | `gpt-image` 2.0; 29.07.2026; gesteuert mit `gpt-5.6-sol` | Chroma-Key mit weicher Matte entfernt, beschnitten, auf höchstens 512 × 512 skaliert und als Alpha-WebP exportiert; `candy_overheat` 768 × 768 | CC BY-NC-SA 4.0; Gerry Weißbach (gamma / gamma production) |

Der Neon-Basis-Prompt beschreibt somit reproduzierbar die sichtbare
Bildsprache. Die historischen Einzelprompts sind zusätzlich in der lokalen
Codex-Sitzung vom 28./29.07.2026 erhalten; der Basis-Prompt ist weiterhin eine
wartbare Zusammenfassung und kein behaupteter wortgleicher Originalprompt.

### Rekonstruierte Produktionsfolge

1. Am 28.07.2026 entstanden die drei textbasierten Classic-Neon-Anker
   `countup`, `x01` und `cricket`.
2. Am 29.07.2026 entstanden zehn weitere Neon-Cover mit ausschließlich diesen
   drei Projektbildern als Stilreferenzen.
3. `heart_chase` wurde anschließend in zwei textbasierten Varianten erzeugt.
   Die ausgewählte zweite Variante wurde zum verbindlichen Cartoon-Anker.
4. Die übrigen 23 Cartoon-Cover wurden mit diesem projektinternen
   `heart_chase`-Bild als einziger Bildreferenz erzeugt.
5. Zwölf Ambient-Props wurden einzeln vor Grün beziehungsweise Magenta erzeugt
   und lokal freigestellt. Der spätere Candy-Overheat-Effekt verwendete nur das
   eigene Candy-Cannon-Cover und den eigenen Candy-Prop als Referenzen.

Die zugehörigen Git-Commits sind `179cf40` für die ersten drei Cover,
`e5f3192` für die ergänzten Neon-Artworks, `475d23f` für das vollständige
Cartoon-Pack, `84b65ba` für die Wiederherstellung des Neon-Packs, `ef75121` für
die zwölf Ambient-Props und `bd67109` für `candy_overheat`.

### Website-Screenshots

Die fünf JPEGs unter `website/assets/screenshots/` sind keine
ImageGen-Ergebnisse. Sie wurden am 29.07.2026 mit
`website/capture-screenshots.mjs` aus einer frischen lokalen Spielsession
aufgenommen:

- Browser: Playwright mit WebKit, Device Scale Factor 1, dunkles Farbschema,
- Controller-Viewports: 1440 × 1000 und 1280 × 900,
- Projektor-Viewports: 1600 × 900 und 1920 × 1080,
- Format: JPEG mit Qualität 88,
- Zustände: echte lokale API-Session mit drei Testspielern und fest
  protokollierten Testtreffern; keine statischen UI-Mockups.

Die Screenshots kamen mit Commit `e01e095` in das Repository. Sie bilden
allerdings die Spielmodus-Artworks ab und können deshalb erst zusammen mit den
dargestellten Assets abschließend lizenziert werden.

## Animierte 3D-Props

Die Ambient-Animationen verwenden freigestellte Einzelobjekte aus
`web/static/assets/effects/`. Als Stilreferenz dient ebenfalls
`web/static/assets/modes/heart_chase.webp`. Der gemeinsame Prompt lautet:

```text
Use case: stylized-concept
Asset type: isolated high-resolution 3D prop sprite for a smart dart arcade projector
Primary request: Render one large, instantly recognizable [SUBJECT] matching the joyful handcrafted 3D cartoon style of the supplied Heart Chase reference.
Style/medium: polished cinematic 3D animation render; tactile painted wood, soft fabric, clay or toy-like material appropriate to the subject; rounded friendly shapes; believable depth and fine surface detail.
Composition/framing: a single complete object centered in a square canvas, three-quarter view, generous clean margin, no cropping, readable when displayed at 50–100 px.
Lighting/mood: warm soft studio key light, gentle contact shading on the object itself, cheerful arcade mood, crisp silhouette.
Background: perfectly flat saturated chroma-key green or magenta, uniformly lit, with no floor, horizon, cast shadow or environmental reflection.
Constraints: exactly one object; no dartboard; no people; no text; no letters; no logo; no watermark; no border.
Avoid: emojis, icons, flat vector art, multiple objects, scenery, dark nightclub style, neon tubes, bloom obscuring the silhouette, transparent-looking holes except where physically required.
```

Nach der Generierung wird die Chroma-Fläche mit einer weichen Matte entfernt,
das Ergebnis auf maximal `512 × 512 px` skaliert und als Alpha-WebP mit
Qualitätsstufe 90 gespeichert. Die aktuelle Bibliothek umfasst:

| Asset | Motiv | Verwendete Effekte |
|---|---|---|
| `heart.webp` | Stoffherz | Heart Chase |
| `egg.webp` | bemaltes Spielzeugei | Dragon Eggs |
| `cookie.webp` | weicher Schoko-Cookie | Cookie Monster |
| `candy.webp` | eingewickeltes Bonbon | Candy Cannon |
| `block.webp` | abgerundeter Puzzleblock | Block Drop |
| `billiard.webp` | schwarze Achterkugel | Eight Ball |
| `golf.webp` | Golfball | Mini Golf |
| `wisp.webp` | freundlicher Geisterschweif | Ghost Chase |
| `leaf.webp` | weiches Eichenblatt | Cricket, Robin Hood |
| `mine.webp` | harmlose Spielzeugmine | Avoid Bomb, Dart Sweeper |
| `coin.webp` | goldene Sternmünze | Risk It |
| `gem.webp` | blauer Spielzeugedelstein | Treasure Hunt |
| `candy_overheat.webp` | Zuckerstaub-Explosion mit Bonbons und Konfetti | Candy Cannon Overheat und kleiner FIRE-Einschlag |

Für `candy_overheat.webp` wird `[SUBJECT]` im gemeinsamen Prop-Prompt durch
folgende Effektspezifikation ersetzt:

```text
A compact radial Candy Cannon overheat explosion: a large burst-shaped cloud
of thick cream-colored sugar dust and cotton-candy puffs with a hot coral-orange
center, several small wrapped candies and colorful paper confetti bursting
outward. Energetic and unmistakably explosive, but playful, family-friendly
and made from tactile felt, painted clay and paper. Keep the complete silhouette
inside the square canvas with generous margin. No cannon, dartboard, text,
realistic fire, dark soot or translucent smoke.
```
