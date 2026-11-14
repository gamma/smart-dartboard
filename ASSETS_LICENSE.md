# Visual Asset Rights and Provenance

## Proprietary rights reservation

Project-owned visual assets and screenshots are proprietary. All rights
reserved, to the extent held by Gerry Weißbach (gamma / gamma production).
See [LICENSE](LICENSE). Neither private nor commercial reuse is granted by
this file. A separate written agreement is required, subject to applicable law.
Viewing screenshots on the public website does not grant reuse rights.
Third-party materials retain their own terms; provenance records do not
establish rights beyond those actually held.

## Provenance clearance

The table below records provenance clearance only. An entry does not grant a
public license or authorize publication, distribution, or commercial use.
Assets without a cleared entry still require provenance review.

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
| `web/static/assets/themes/neon/effects/*.webp` | Gerry Weißbach (gamma / gamma production) | OpenAI `gpt-image` 2.0; only project-owned Classic-Neon covers and effect sprites as references; exact prompts and processing in `docs/NEON_EFFECTS_2026-07-30.md`; provenance commit `74865e6` | 2026-07-30 |
| `web/static/assets/effects/{dragon_scale,dragon_fire}.webp` and `web/static/assets/themes/neon/effects/{dragon_scale,dragon_fire}.webp` | Gerry Weißbach (gamma / gamma production) | OpenAI `gpt-image` 2.0; only project-owned theme anchors, Dragon-Eggs covers and egg sprites as references; exact prompts and processing in `docs/DRAGON_EGGS_ASSETS_2026-07-30.md`; provenance commit `bede470` | 2026-07-30 |
| `website/assets/modes/*.webp` | Gerry Weißbach (gamma / gamma production) | Copies of the cleared Cartoon covers | 2026-07-30 |
| `website/assets/neon/*.webp` | Gerry Weißbach (gamma / gamma production) | Copies of the cleared Classic-Neon covers | 2026-07-30 |
| `website/assets/screenshots/*.jpg` | Gerry Weißbach (gamma / gamma production) | Local Playwright/WebKit captures of the cleared project UI and artworks; reproducible capture commit `f323050` | 2026-07-31 |

Entries cover the files present at provenance commit `0069737`. A later file
matching one of these globs must receive its own provenance review before it is
treated as cleared.

## Provenance summary

The cleared graphics were generated locally with OpenAI ImageGen, with the
workflow orchestrated by GPT-5.6-sol. The local generation records and
surviving source PNGs identify the generator as OpenAI `gpt-image` version 2.0.
The repository contains 84 selected ImageGen assets: 48 covers and 36
theme-specific effects.
All supplied image references point to assets created earlier within this
project; no external image reference appears in the recorded calls. The
Website screenshots were captured locally with Playwright and WebKit. Detailed
provenance notes are maintained in `docs/ARTWORK_PROMPTS.md`.

## Excluded brand assets

Names, logos, favicon and distinctive brand identifiers remain reserved.
See [TRADEMARKS.md](TRADEMARKS.md).

## Separate licensing

Customer use of cleared assets may be authorized by a separate written
agreement with the applicable rights holder. Valid rights in any previously
distributed copies are unaffected; this file makes no claim that such a
prior distribution occurred.
