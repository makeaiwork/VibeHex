"""Tests for scripts/export-preset: only allow-listed keys leave settings.json."""

import importlib.machinery
import importlib.util
import json
import pathlib
import unittest

SCRIPT = pathlib.Path(__file__).resolve().parent / "export-preset"
LOADER = importlib.machinery.SourceFileLoader("export_preset", str(SCRIPT))
SPEC = importlib.util.spec_from_loader("export_preset", LOADER)
export_preset = importlib.util.module_from_spec(SPEC)
LOADER.exec_module(export_preset)

SECRETS = [
    "sk-or-v1-secret",
    "jev-secret-key",
    "http://127.0.0.1:1234/v1",
    "/Users/someone/Meetings",
    "/Users/someone/Captures",
    "MacBook Pro Microphone",
    "work@example.com",
    "com.tinyspeck.slackmacgap",
    "github.com",
    "nightly",
]

FIXTURE = {
    "microphone": "MacBook Pro Microphone",
    "shortcut_mode": "toggle",
    "double_tap_lock": True,
    "dictation_hotkey": {"keys": [{"modifier": "option", "side": "right"}]},
    "stop_hotkey": {"keys": [{"code": 36, "label": "Return"}]},
    "raw_dictation_hotkey": None,
    "sound_effects": False,
    "transcription": {"model": "orukeet", "language": "ru"},
    "transcription_recents": ["orukeet", "parakeet_v3"],
    "dictation_processing": {
        "default_mode": {
            "name": "Global",
            "applications": ["com.tinyspeck.slackmacgap"],
            "browser_hosts": ["github.com"],
            "post_processing": {"enabled": True, "prompt": "Keep English terms."},
        },
        "modes": [{"name": "Slack", "applications": ["com.tinyspeck.slackmacgap"]}],
        "server": {
            "runs_on": "local",
            "url": "http://127.0.0.1:1234/v1",
            "openrouter_api_key": "sk-or-v1-secret",
            "local_model": "gemma-4-e4b-it",
            "codex_model": "gpt-6.1-terra",
            "codex_effort": "low",
        },
    },
    "meetings": {
        "enabled": True,
        "output_folder": "/Users/someone/Meetings",
        "retention": "month",
        "providers": ["zoom"],
        "calendar": {"connected": True, "excluded_calendars": ["work@example.com"]},
        "notes": {"writer": "codex:gpt-6.1-terra", "effort": "medium"},
    },
    "capture": {
        "folder": "/Users/someone/Captures",
        "routing": {"jevApiKey": "jev-secret-key", "jev": "https://jev.example"},
    },
    "files": {"cleanup_engine": "codex", "last_folder": "/Users/someone/Captures"},
    "assistant": {"engine": "codex", "read_recordings": True},
    "labs": {"russian_vibe_coding": True, "files": True, "sidebar_collapsed": True},
    "updates": {"channel": "nightly"},
    "migrations": ["a", "b"],
    "vocabulary": [
        {"term": "Claude Code", "sounds_like": ["клод код"]},
        "GitHub",
        {"term": " "},
        {"term": "Monki", "sounds_like": ["монки"]},
        "MONKI",
        {"term": " monki "},
    ],
}


class ExportTest(unittest.TestCase):
    def setUp(self):
        self.preset = export_preset.export(FIXTURE)
        self.text = json.dumps(self.preset, ensure_ascii=False)

    def test_no_secret_path_or_personal_value_survives(self):
        for secret in SECRETS:
            self.assertNotIn(secret, self.text)

    def test_no_denied_key_survives(self):
        for part in ("settings", "shortcuts"):
            for key in self.preset[part]:
                self.assertIsNone(export_preset.DENIED.search(key), key)
        for key in ("microphone", "labs.sidebar_collapsed", "assistant.engine",
                    "meetings.providers", "dictation_processing.server.url",
                    "transcription_recents", "sound_effects"):
            self.assertNotIn(key, self.preset["settings"])
            self.assertNotIn(key, self.preset["shortcuts"])

    def test_what_a_preset_carries(self):
        settings = self.preset["settings"]
        self.assertEqual(settings["transcription.model"], "orukeet")
        self.assertEqual(settings["dictation_processing.server.runs_on"], "local")
        self.assertEqual(settings["meetings.notes.writer"], "codex:gpt-6.1-terra")
        self.assertEqual(settings["meetings.retention"], "month")
        self.assertEqual(settings["labs.russian_vibe_coding"], True)
        self.assertEqual(settings["dictation_processing.default_mode.cleanup"], True)
        self.assertEqual(self.preset["prompt"], "Keep English terms.")
        self.assertEqual(
            self.preset["vocabulary"],
            [{"term": "Claude Code", "sounds_like": ["клод код"]}, {"term": "GitHub"}],
        )
        self.assertEqual(
            sorted(self.preset["shortcuts"]),
            ["dictation_hotkey", "double_tap_lock", "shortcut_mode", "stop_hotkey"],
        )

    def test_every_kept_key_is_in_the_registry(self):
        keys = export_preset.registry()
        for key in self.preset["settings"]:
            if key != "dictation_processing.default_mode.cleanup":
                self.assertIn(key, keys)

    def test_the_owners_private_terms_stay_with_him(self):
        # His own app's name, in any case, as an entry or a plain term; a
        # term that only holds the word is another term.
        self.assertIn("monki", export_preset.PRIVATE_TERMS)
        self.assertNotIn("monki", self.text.casefold())
        self.assertNotIn("монки", self.text)
        longer = dict(FIXTURE, vocabulary=["Monki Pro", "Monkit"])
        kept = [term["term"] for term in export_preset.export(longer)["vocabulary"]]
        self.assertEqual(kept, ["Monki Pro", "Monkit"])

    def test_the_pasteable_list_is_the_presets_vocabulary(self):
        text = export_preset.pasteable(self.preset["vocabulary"])
        lines = [line for line in text.splitlines() if line and not line.startswith("#")]
        self.assertEqual(lines, ["Claude Code | клод код", "GitHub"])
        committed = json.loads((export_preset.ROOT / "presets/deniskey.json").read_text())
        self.assertEqual(
            export_preset.DEFAULT_LIST.read_text(),
            export_preset.pasteable(committed["vocabulary"]),
        )

    def test_the_committed_preset_is_clean(self):
        committed = export_preset.ROOT / "presets/deniskey.json"
        preset = json.loads(committed.read_text())
        for part in ("settings", "shortcuts"):
            for key in preset[part]:
                self.assertIsNone(export_preset.DENIED.search(key), key)
                self.assertIsNotNone(export_preset.wanted(key, export_preset.registry().get(key, "Bool"))
                                     or key == "dictation_processing.default_mode.cleanup", key)
        terms = [term["term"].casefold() for term in preset["vocabulary"]]
        self.assertEqual(len(terms), 90)
        for private in export_preset.PRIVATE_TERMS:
            self.assertNotIn(private, terms)


if __name__ == "__main__":
    unittest.main()
