# Classic-Neon-Frontassets vom 30.07.2026

Dieses Dokument protokolliert die 16 Frontgrafiken des Classic-Neon-Packs.
Sie liegen unter `web/static/assets/themes/neon/effects/` und ersetzen keine
Playful-Cartoon-Datei.

## Erzeugung

- Generator: eingebautes OpenAI ImageGen
- Bildmodell laut C2PA-Ausgabe: `gpt-image` 2.0
- Orchestrierung: `gpt-5.6-sol`
- Generierungsdatum: 30.07.2026
- Primäre Stilreferenz: `web/static/assets/themes/neon/modes/countup.webp`
- Zweite Stilreferenz: das jeweils nächstliegende projektinterne Neon-Cover
- Dritte Referenz: der gleichnamige projektinterne Cartoon-Prop, ausschließlich
  für Silhouette und spielerische Bedeutung
- Fremde Bilder, Personen, Marken, Logos und lizenzierte Figuren: keine

Jedes Asset wurde in einem eigenen ImageGen-Aufruf erzeugt. Der gemeinsame
wortgleiche Basisprompt war:

```text
Create one isolated high-resolution 3D arcade game prop sprite in the exact visual language of the supplied Classic Neon dart-game artwork: lifelike polished materials, playful arcade personality, cinematic electric-blue rim lighting with restrained cyan highlights, deep realistic shadows, premium game-render finish, strong readable silhouette, centered full object, front three-quarter view. Flat solid #ff00ff chroma-key background, no floor, no scenery, no border, no text, no letters, no numbers, no logo, no watermark. Keep generous clean space around the complete object; nothing cropped.
```

Für `mine_explosion` und `candy_overheat` wurde im ersten Satz lediglich
`prop sprite` durch `effect sprite` ersetzt und am Ende `or effect` ergänzt.

## Exakte Subjektzusätze und Referenzen

| Datei | Wortgleicher Zusatz nach `Subject:` | Moduscover |
|---|---|---|
| `heart.webp` | a charming glossy ruby-red heart pickup, slightly faceted and dimensional, energetic but not cute-cartoon flat. | `heart_chase.webp` |
| `egg.webp` | one mysterious dragon egg pickup, dark indigo shell with luminous cyan cracks and subtle metallic scales. | `dragon_eggs.webp` |
| `cookie.webp` | one appetizing round chocolate-chip cookie pickup, richly baked texture, dimensional chips, warm amber highlights balanced by blue neon rim light. | `cookie_monster.webp` |
| `cookie_moldy.webp` | one clearly spoiled moldy round chocolate-chip cookie hazard, cracked dark baked texture with visible sickly teal-green mold patches, still readable and playful rather than disgusting. | `cookie_monster.webp` |
| `milk.webp` | a small old-fashioned clear glass milk bottle pickup filled with bright white milk, sealed with a glossy cyan-blue cap. | `cookie_monster.webp` |
| `candy.webp` | one colorful wrapped hard-candy ammunition pickup, glossy striped candy center, metallic translucent twisted wrapper ends, joyful and immediately readable. | `candy_cannon.webp` |
| `block.webp` | one compact interlocking arcade puzzle block cluster made of four beveled cubes, electric cyan glass and brushed dark metal, no symbols. | `block_drop.webp` |
| `billiard.webp` | one glossy black billiard eight-ball style game prop but with absolutely no numeral or text, recognizable by its spherical polished billiard-ball material and a small plain white circular inset. | `eight_ball.webp` |
| `golf.webp` | one clean white dimpled golf ball pickup with a small brushed-metal electric-blue tee beneath it, compact single sprite. | `mini_golf.webp` |
| `wisp.webp` | one mischievous spectral wisp pickup, compact floating blue-white flame with two tiny luminous eyes, volumetric but with a crisp contained silhouette and no detached particles. | `ghost_chase.webp` |
| `leaf.webp` | one elegant emerald-green leaf pickup, subtly faceted realistic leaf surface, visible gold-edged veins and a short stem, isolated single leaf. | `robin_hood.webp` |
| `mine.webp` | one dangerous compact futuristic proximity mine, dark brushed steel sphere with short blunt studs and a bright red-orange armed core, readable at small size, no explosion and no smoke. | `dart_sweeper.webp` |
| `coin.webp` | one thick premium arcade coin pickup, brushed warm gold metal with beveled concentric rings and a plain glowing cyan center inset; absolutely no currency mark or symbol. | `risk_it.webp` |
| `gem.webp` | one faceted treasure gemstone pickup, deep emerald-green crystal held in a minimal dark-metal setting, cyan rim highlights, no detached sparkles. | `treasure_hunt.webp` |
| `mine_explosion.webp` | a single contained mine detonation sprite: powerful orange-white fireball with a compact circular shockwave, a few dark-metal fragments and restrained dark smoke; dramatic but all flame, smoke, and debris remain fully inside the canvas with clean separation from the chroma background. | `dart_sweeper.webp` |
| `candy_overheat.webp` | a single contained candy-cannon overheat burst sprite: playful pressure explosion with vivid orange-white fire at the center, electric-cyan shock ring, several recognizable wrapped candies bursting outward, a little dark smoke; energetic and humorous, but every element remains fully inside the canvas. | `candy_cannon.webp` |

Die dritte Referenz hatte jeweils denselben Dateinamen unter
`web/static/assets/effects/`. Die beiden Burst-Referenzen waren entsprechend
`mine_explosion.webp` und `candy_overheat.webp`.

## Nachbearbeitung

Die magentafarbene Fläche wurde mit dem projektlokal verwendeten
`remove_chroma_key.py` entfernt:

- automatische Farbabnahme aus den Ecken,
- weiche Matte mit transparentem Schwellwert 32 und opakem Schwellwert 105,
- Matte um 1 Pixel kontrahiert, 0,6 Pixel Kantenweichzeichnung,
- Magenta-Despill.

Danach wurden die sichtbaren Inhalte beschnitten, proportional skaliert,
zentriert und mit ImageMagick als Alpha-WebP mit Qualität 90 exportiert.
Normale Props verwenden `512 × 512`, `mine_explosion` und `candy_overheat`
`768 × 768`.

## Auswahlprüfung

Alle 16 Ergebnisse wurden vor der Übernahme einzeln und gemeinsam auf einem
Schachbretthintergrund geprüft. Die Silhouetten sind vollständig lesbar,
enthalten keine Schrift oder Marken und besitzen transparente Außenbereiche.
Der Theme-Resolver wechselt Cover, Ambient-Props, Overlay-Icons,
Candy-Projektil und beide großen Explosionen gemeinsam.
