# Visual Asset License

## Intended license

Project-owned visual assets that have a cleared provenance entry are licensed
under the
[Creative Commons Attribution-NonCommercial-ShareAlike 4.0 International
License](https://creativecommons.org/licenses/by-nc-sa/4.0/).

Required attribution:

```text
Smart Dartboard visual assets © 2026 Gerry Weißbach
(gamma / gamma production) — CC BY-NC-SA 4.0
https://github.com/gamma/smart-dartboard
```

This license permits sharing and adaptations for non-commercial purposes when
attribution is retained and adaptations use the same license. Commercial use,
including use in a revenue-generating arcade installation, requires a separate
written license from the applicable rights holder.

## Activation requires cleared provenance

An asset is covered by CC BY-NC-SA 4.0 only when its path is explicitly listed
in the **Cleared assets** table below. An absent entry grants no permission.
This prevents the repository from offering rights it may not own.

### Cleared assets

| Path or glob | Rights holder | Source and references | Cleared on |
|---|---|---|---|
| `web/static/assets/modes/*.webp` | Gerry Weißbach (gamma / gamma production) | OpenAI `gpt-image` 2.0; text-only Cartoon anchor and project-internal references; provenance commit `0069737` | 2026-07-30 |
| `web/static/assets/themes/neon/modes/*.webp` | Gerry Weißbach (gamma / gamma production) | OpenAI `gpt-image` 2.0; text-only Neon anchors and project-internal references; provenance commit `0069737` | 2026-07-30 |
| `web/static/assets/effects/*.webp` | Gerry Weißbach (gamma / gamma production) | OpenAI `gpt-image` 2.0; project-internal references and local chroma-key processing; provenance commit `0069737` | 2026-07-30 |
| `web/static/assets/effects/cookie_moldy.webp` | Gerry Weißbach (gamma / gamma production) | OpenAI `gpt-image` 2.0; `heart_chase.webp` and `cookie.webp` as project-internal references; provenance commit `88ee789` | 2026-07-30 |
| `web/static/assets/effects/milk.webp` | Gerry Weißbach (gamma / gamma production) | OpenAI `gpt-image` 2.0; `heart_chase.webp` and `cookie.webp` as project-internal references; provenance commit `88ee789` | 2026-07-30 |
| `web/static/assets/effects/mine.webp` | Gerry Weißbach (gamma / gamma production) | OpenAI `gpt-image` 2.0; `heart_chase.webp` and the previous project-owned mine as internal references; provenance commit `fc87adc` | 2026-07-30 |
| `web/static/assets/effects/mine_explosion.webp` | Gerry Weißbach (gamma / gamma production) | OpenAI `gpt-image` 2.0; `heart_chase.webp` and the project-owned mine as internal references; provenance commit `fc87adc` | 2026-07-30 |
| `web/static/assets/themes/neon/modes/{block_drop,candy_cannon,cookie_monster,dart_sweeper,dragon_eggs,eight_ball,ghost_chase,heart_chase,mini_golf,robin_hood,space_defender}.webp` | Gerry Weißbach (gamma / gamma production) | OpenAI `gpt-image` 2.0; only project-owned Classic-Neon covers as references; exact prompts in `docs/NEON_ARTWORK_2026-07-30.md`; provenance commit `8e2ee93` | 2026-07-30 |
| `website/assets/modes/*.webp` | Gerry Weißbach (gamma / gamma production) | Copies of the cleared Cartoon covers | 2026-07-30 |
| `website/assets/neon/*.webp` | Gerry Weißbach (gamma / gamma production) | Copies of the cleared Classic-Neon covers | 2026-07-30 |
| `website/assets/screenshots/*.jpg` | Gerry Weißbach (gamma / gamma production) | Local Playwright/WebKit captures of the cleared project UI and artworks | 2026-07-30 |

Entries cover the files present at provenance commit `0069737`. A later file
matching one of these globs must receive its own provenance review before it is
treated as cleared.

## Provenance summary

The cleared graphics were generated locally with OpenAI ImageGen, with the
workflow orchestrated by GPT-5.6-sol. The local generation records and
surviving source PNGs identify the generator as OpenAI `gpt-image` version 2.0.
The repository contains 64 selected ImageGen assets: 48 covers and 16 effects.
All supplied image references point to assets created earlier within this
project; no external image reference appears in the recorded calls. The
Website screenshots were captured locally with Playwright and WebKit. Detailed
provenance notes are maintained in `docs/ARTWORK_PROMPTS.md`.

## Excluded brand assets

The project name, word marks, logos, favicon, and distinctive brand identifiers
are not licensed under CC BY-NC-SA 4.0. See `TRADEMARKS.md`.

## Commercial licensing

Gerry Weißbach (gamma / gamma production) may offer the same cleared assets
under separate commercial terms. The non-commercial Creative Commons grant
remains in force for copies already distributed under it.
