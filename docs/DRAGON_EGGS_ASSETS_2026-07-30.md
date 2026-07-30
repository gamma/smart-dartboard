# Dragon-Eggs-Frontassets vom 30.07.2026

Für Dragon Eggs wurden vier eigenständige Reaktionsgrafiken ergänzt:

- `web/static/assets/effects/dragon_scale.webp`
- `web/static/assets/effects/dragon_fire.webp`
- `web/static/assets/themes/neon/effects/dragon_scale.webp`
- `web/static/assets/themes/neon/effects/dragon_fire.webp`

Generator war das eingebaute OpenAI ImageGen, das C2PA-Manifest nennt
`gpt-image` 2.0. Die Generierung wurde mit `gpt-5.6-sol` gesteuert. Sämtliche
Referenzen stammen aus diesem Projekt.

## Playful Cartoon

Referenzen für beide Ergebnisse:

1. `web/static/assets/modes/heart_chase.webp` als verbindlicher Stilmanker,
2. `web/static/assets/modes/dragon_eggs.webp` für die Spielwelt,
3. `web/static/assets/effects/egg.webp` für Maßstab und Material.

Finaler Prompt für `dragon_scale.webp`:

```text
Use case: stylized-concept
Asset type: isolated high-resolution 3D hazard sprite for Smart Dartboard Dragon Eggs
Input images: Image 1 is the mandatory Playful Cartoon style reference; Image 2 establishes the Dragon Eggs world; Image 3 establishes the existing egg prop's scale and material language.
Primary request: Render one large unmistakable dragon scale hazard as a premium handcrafted arcade prop.
Subject: exactly one broad shield-shaped dragon scale, warm coral-red padded felt with a mustard-gold raised rim, subtle stitched diamond texture and one tiny ember-orange glow near its lower edge; harmless and playful but clearly dangerous.
Style/medium: polished cinematic stylized 3D animation render, tactile felt, plush padding, painted clay trim, rounded family-friendly shapes, believable depth and crisp surface detail.
Composition/framing: one complete scale centered in a square canvas, slight three-quarter front view, generous clean margin, no cropping, strong readable silhouette at 25–80 px.
Lighting/mood: warm soft studio key light, restrained ember rim, cheerful fantasy arcade mood.
Background: perfectly flat solid #00ff00 chroma-key green, uniformly lit, no floor, horizon, cast shadow, gradient, texture or reflection.
Constraints: no egg, dragon, fire, dartboard, people, face, text, letters, numbers, logo, brand, watermark or border; do not use #00ff00 in the subject.
Avoid: realistic reptile skin, gore, sharp weapon shapes, emoji, flat vector art, dark nightclub scene, heavy bloom.
```

Finaler Prompt für `dragon_fire.webp`:

```text
Use case: stylized-concept
Asset type: isolated high-resolution 3D reaction sprite for Smart Dartboard Dragon Eggs
Input images: Image 1 is the mandatory Playful Cartoon style reference; Image 2 establishes the Dragon Eggs world; Image 3 establishes the tactile egg prop.
Primary request: Render one large contained playful dragon-fire burst that instantly reads as the dragon awakening.
Subject: a compact curling flame plume built from opaque padded felt and painted clay, bright warm-cream core, mustard-yellow and coral-orange flame layers, three small rounded charcoal smoke puffs and a few harmless golden sparks attached close to the plume.
Style/medium: premium animated-film 3D render with handcrafted tactile materials, rounded family-friendly forms, rich depth and crisp layered silhouette.
Composition/framing: one complete roughly vertical flame burst centered in a square canvas, generous margin on every side, no cropping, readable at 100–500 px.
Lighting/mood: energetic warm fantasy arcade celebration, luminous center but restrained glow that preserves material texture.
Background: perfectly flat solid #00ff00 chroma-key green, uniformly lit, no floor, horizon, cast shadow, gradient, texture or reflection.
Constraints: all flame and smoke forms opaque for chroma removal; no dragon, egg, dartboard, people, face, text, letters, numbers, logo, brand, watermark or border; do not use #00ff00 in the effect.
Avoid: photoreal fire simulation, translucent smoke, destruction, horror, emoji, flat vector starburst, heavy bloom.
```

## Classic Neon

Referenzen für beide Ergebnisse:

1. `web/static/assets/themes/neon/modes/countup.webp` als Stilmanker,
2. `web/static/assets/themes/neon/modes/dragon_eggs.webp` für die Spielwelt,
3. `web/static/assets/themes/neon/effects/egg.webp` für Maßstab und Material.

Finaler Prompt für `dragon_scale.webp`:

```text
Create one isolated high-resolution 3D arcade hazard sprite in the exact visual language of the supplied Classic Neon dart-game artwork: lifelike polished materials, premium dark-metal finish, cinematic electric-blue rim lighting, restrained bloom, strong readable silhouette. Subject: exactly one broad shield-shaped dragon scale made from deep ruby-red translucent mineral glass in a blackened metal rim, engraved with a subtle scale texture and a small amber heat core. Centered full object, front three-quarter view. Flat solid #00ff00 chroma-key background, no floor, no scenery, no border, no text, no letters, no numbers, no egg, no dragon, no fire, no logo, no watermark. Generous clean space, nothing cropped; do not use chroma green in the subject.
```

Finaler Prompt für `dragon_fire.webp`:

```text
Create one isolated high-resolution 3D arcade reaction sprite in the exact visual language of the supplied Classic Neon dart-game artwork: premium cinematic game-render finish, realistic volumetric depth expressed with mostly opaque layered forms, electric-blue rim lighting and restrained bloom. Subject: one contained curling dragon-fire burst with a brilliant amber-white core, layered orange and deep-red flame petals, a compact cyan shock ring and a few blackened-metal scale fragments; dramatic, playful arcade energy, not destructive or horrific. Entire effect centered and fully inside the square canvas. Flat solid #ff00ff chroma-key background, no floor, no scenery, no border, no text, no letters, no numbers, no dragon, no egg, no logo, no watermark.
```

## Nachbearbeitung

Die Chroma-Flächen wurden mit `remove_chroma_key.py` automatisch aus den
Ecken ermittelt, mit weicher Matte, einem Pixel Kantenkontraktion,
0,4 Pixel Kantenweichzeichnung und Despill entfernt. Die sichtbaren Inhalte
wurden proportional skaliert, zentriert und als Alpha-WebP mit Qualität 90
gespeichert. Schuppen verwenden `512 × 512`, Feuerbursts `768 × 768`.
