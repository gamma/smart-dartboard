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
