from __future__ import annotations

from html.parser import HTMLParser
from pathlib import Path
import re
import unittest

from sdb_dartboard.games import registry


ROOT = Path(__file__).resolve().parents[1]
WEBSITE = ROOT / "website"


class _AssetParser(HTMLParser):
    def __init__(self) -> None:
        super().__init__()
        self.references: list[str] = []

    def handle_starttag(self, tag, attrs):
        del tag
        for key, value in attrs:
            if key in {"href", "src"} and value:
                self.references.append(value)


class WebsiteContractTests(unittest.TestCase):
    def test_local_website_references_exist(self):
        parser = _AssetParser()
        parser.feed((WEBSITE / "index.html").read_text())
        for reference in parser.references:
            if reference.startswith(("http://", "https://", "#")):
                continue
            path = reference.split("#", 1)[0].split("?", 1)[0]
            candidate = WEBSITE / path.removeprefix("./")
            if path in {"./ASSETS_LICENSE.md", "./LICENSE", "./NOTICE", "./TRADEMARKS.md"}:
                candidate = ROOT / path.removeprefix("./")
            with self.subTest(reference=reference):
                self.assertTrue(candidate.is_file(), candidate)

    def test_published_mode_count_matches_registry(self):
        html = (WEBSITE / "index.html").read_text()
        mode_count = len(list(registry.all()))
        self.assertIn(f"<strong>{mode_count}</strong><span>Spielmodi</span>", html)
        self.assertIn(f"{mode_count} MODI. EINE SCHEIBE.", html)

    def test_every_mode_has_both_published_cover_themes(self):
        slugs = {mode.metadata.slug for mode in registry.all()}
        for directory in (
            ROOT / "web/static/assets/modes",
            ROOT / "web/static/assets/themes/neon/modes",
        ):
            self.assertEqual(slugs, {path.stem for path in directory.glob("*.webp")})

    def test_website_language_defaults_to_browser_and_remains_switchable(self):
        html = (WEBSITE / "index.html").read_text()
        javascript = (WEBSITE / "app.js").read_text()
        self.assertEqual(2, len(re.findall(r"data-site-language=", html)))
        self.assertIn("navigator.language", javascript)
        self.assertIn("startsWith('de')?'de':'en'", javascript)
        self.assertIn("localStorage.setItem(SITE_LANGUAGE_KEY", javascript)


if __name__ == "__main__":
    unittest.main()
