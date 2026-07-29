# Reproduzierbare Spielmodus-Artworks

Die Cover unter `web/static/assets/modes/<slug>.webp` verwenden dieselbe
Bildsprache wie die ursprünglichen Cover für Count Up, X01 und Cricket:
realistische, hochwertige 3D-/Product-Renderings eines echten Dartboards in
einer dunklen Spielhallen- oder Bühnenumgebung.

## Referenzbilder

Bei einer Neugenerierung immer gemeinsam als Stilreferenz verwenden:

- `web/static/assets/modes/countup.webp`
- `web/static/assets/modes/x01.webp`
- `web/static/assets/modes/cricket.webp`

Die Referenzen bestimmen Materialtreue, Licht, Kontrast, Tiefenschärfe und
Kamerawirkung. Motive und Farbakzente kommen aus dem jeweiligen Modus-Prompt.

## Basis-Prompt

```text
Use case: stylized-concept
Asset type: cinematic game-mode cover for a touch-controlled smart dart arcade
Primary request: Create a playful arcade interpretation of the specified game mode while matching the three supplied classic dartboard cover references.
Input images: the Count Up, X01 and Cricket covers are style references only.
Scene/backdrop: a believable premium arcade, club or arena environment, very dark and atmospheric, with subtle practical lights and haze.
Subject: one physically plausible regulation dartboard remains the unmistakable hero object; add only a few mode-specific physical or holographic cues around it.
Style/medium: lifelike high-end cinematic 3D product render, realistic sisal, metal wire, darts, glass and light; playful but not cartoon, not flat graphic design.
Composition/framing: landscape 3:2; dartboard centered slightly right; clear darker negative space on the left and lower-left for UI copy; strong depth and perspective; no important detail at the outer edges.
Lighting/mood: dramatic volumetric rim light, controlled neon accents, deep blacks, premium arcade energy, crisp focal board, shallow depth of field.
Color palette: mostly black and charcoal with the mode-specific accent colors; preserve natural cream, black, red and green dartboard materials.
Constraints: one coherent scene; regulation board geometry; visually readable at card size; no people; no logos; no brands; no watermark; no frame; no UI.
Avoid: any text, letters, numbers outside the dartboard, title typography, poster layout, flat circles, vector art, 2D illustration, oversized abstract blobs, duplicated boards, deformed darts, fantasy board geometry, childish cartoon styling.
```

## Modusspezifische Ergänzungen

| Slug | Motiv und Akzent |
|---|---|
| `target_rush` | Cyan/Grün; ein präzises Segment leuchtet als holografisches Ziel, leichte Bewegungsstreifen fliegender Darts |
| `avoid_bomb` | Magenta/Amber; wenige rote Warnsegmente, kontrollierte Funken und eine kleine stilisierte Energie-Explosion hinter dem Board |
| `color_clash` | Gold/Cyan/Grün/Magenta; einzelne Segmente projizieren farbiges Licht in feinen volumetrischen Strahlen |
| `risk_it` | Amber/Magenta; leuchtender Arcade-Jackpot-Chip beziehungsweise Energie-Pot vor dem Sockel, Spannung und hohe Einsätze |
| `king_of_board` | Violett/Cyan; dezente holografische Gebietsgrenzen und eine kleine metallische Krone oberhalb des Boards |
| `treasure_hunt` | Gold/Grün; einzelne Segmente strahlen wie gefundene Schätze, wenige realistische Münzen und ein edelsteinartiger Lichtakzent |
| `boss_fight` | Magenta/Violett; Board vor einer monumentalen Arcade-Arena, bedrohliche Energie-Silhouette und goldene Schwachpunkte, kein konkretes Monster-Gesicht |
| `darts_bingo` | Gold/Violett; schwebendes dezentes 3×3-Hologrammraster hinter dem Board, einige Felder leuchten als Treffer |
| `lightning_round` | Cyan/Gold; elektrischer Impuls und kurze Lichtspuren, eingefrorene schnelle Bewegung, keine unrealistische Beschädigung |
| `simon_says` | Grün/Violett/Cyan; drei nacheinander pulsierende Segmente mit subtilen Lichtbahnen als Memory-Sequenz |

## Ausgabe

Generierte Bilder werden mittig auf `900 × 640 px` beschnitten und als
verlustbehaftetes WebP mit Qualitätsstufe 88 gespeichert. In das Bild wird kein
Titel eingebrannt; Titel und Beschreibung kommen barrierefrei aus der Web-UI.
