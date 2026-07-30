---
name: smart-dartboard-artwork
description: Create, extend, replace, or review game-mode covers and animated prop sprites for the Smart Dartboard project while preserving its Playful Cartoon and Classic Neon theme packs. Use for artwork prompts, ImageGen work, theme consistency, new mode assets, WebP preparation, visual QA, or artwork publishing checks in this repository.
---

# Smart Dartboard Artwork

Keep every generated asset reproducible, theme-specific, and safe to publish.

## Source of truth

Before generating or reviewing artwork, read
`../../../docs/ARTWORK_PROMPTS.md` completely. Treat it as the canonical source
for both theme prompts, reference images, mode motifs, output specifications,
and provenance requirements. Update that document when a new motif or final
prompt differs from what is recorded there; do not duplicate long prompts in
this skill.

## Select the asset path

- Playful Cartoon cover:
  `web/static/assets/modes/<slug>.webp`
- Classic Neon cover:
  `web/static/assets/themes/neon/modes/<slug>.webp`
- Playful Cartoon transparent prop:
  `web/static/assets/effects/<name>.webp`
- Classic Neon transparent prop:
  `web/static/assets/themes/neon/effects/<name>.webp`

Never overwrite the other theme while producing one pack. Preserve historical
Neon covers unless the user explicitly asks to replace a named asset.

## Workflow

1. Identify the mode slug, gameplay idea, requested theme, and asset type.
2. Read the matching base prompt and mode-specific addition from the source of
   truth.
3. Supply the documented style references to ImageGen:
   - Cartoon: `heart_chase.webp` is mandatory.
   - Neon: use `countup.webp` for the core look plus the closest documented
     accent reference.
4. Generate candidates without titles, UI, logos, brands, or watermarks. Keep
   the dartboard recognizable and reserve the documented UI-safe negative
   space.
5. Inspect every candidate visually before replacing a tracked asset. Reject
   broken board geometry, illegible thumbnails, unintended text, duplicated
   boards, mismatched lighting, excessive bloom, or clipped key props.
6. Crop the approved cover to exactly `900 × 640` and encode lossy WebP at
   quality 88. For prop sprites, follow the chroma-key and alpha-WebP procedure
   in the source document.
7. Record the exact final prompt, reference files, generator/model, generation
   date, and any material post-processing in the source document. Never invent
   missing provenance; mark legacy details as unknown.
8. For a new Neon cover, add its slug to `NEON_MODE_ASSETS` in
   `web/static/app.js`. Cartoon covers require no allow-list entry.
9. Run `file <asset>` and verify dimensions and format. Then inspect the mode
   card, instructions, and in-game projector background in WebKit at realistic
   screen sizes.
10. Run `node --check web/static/app.js`, relevant tests, and
    `git diff --check`. Commit artwork and its prompt/provenance record
    together.

## Review gates

Approve only when all gates pass:

- Theme identity is obvious without reading the filename.
- Mode idea is understandable at card size.
- The regulation dartboard remains plausible and is not duplicated.
- Left/lower-left UI remains readable over the cover.
- In-game dimming still leaves the artwork visible without competing with the
  live SVG board.
- No unintended text, trademark, recognizable licensed character, person,
  logo, or watermark appears.
- Final file path, dimensions, prompt record, and theme selection behavior are
  correct.

For publishing reviews, report unknown provenance or licensing status as a
release blocker instead of assuming generated or legacy assets are cleared.
