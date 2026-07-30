from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[1]
NEON_EFFECTS = {
    "heart",
    "egg",
    "cookie",
    "cookie_moldy",
    "milk",
    "candy",
    "block",
    "billiard",
    "golf",
    "wisp",
    "leaf",
    "mine",
    "mine_explosion",
    "coin",
    "gem",
    "candy_overheat",
}


class ArtworkAssetTests(unittest.TestCase):
    def test_neon_effect_pack_is_complete(self):
        asset_dir = ROOT / "web/static/assets/themes/neon/effects"
        self.assertEqual(
            {path.stem for path in asset_dir.glob("*.webp")},
            NEON_EFFECTS,
        )
        self.assertTrue(
            all((asset_dir / f"{name}.webp").stat().st_size > 1_000 for name in NEON_EFFECTS)
        )

    def test_runtime_resolves_neon_front_assets_by_theme(self):
        app_js = (ROOT / "web/static/app.js").read_text()
        effects_css = (ROOT / "web/static/effects.css").read_text()
        interaction_css = (ROOT / "web/static/interaction.css").read_text()

        self.assertIn("/static/assets/themes/neon/effects/", app_js)
        self.assertIn("NEON_EFFECT_ASSETS", app_js)
        self.assertIn("--effect-art", effects_css)
        self.assertIn("--candy-art", interaction_css)
        self.assertNotIn('url("/static/assets/effects/', effects_css)


if __name__ == "__main__":
    unittest.main()
