"""Check formula integrity and release linkage restrictions."""

import contextlib
import hashlib
import io
from pathlib import Path
from types import SimpleNamespace
import tempfile
import unittest
from unittest.mock import patch

import distribution


class DistributionTests(unittest.TestCase):
    def test_formula_pins_each_platform_archive(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            for target in distribution.TARGETS:
                (root / distribution.archive_name(target)).write_bytes(target.encode())
            formula = root / "Formula/memoria.rb"
            with contextlib.redirect_stdout(io.StringIO()):
                distribution.formula(root, formula)
            text = formula.read_text()
            for target in distribution.TARGETS:
                self.assertIn(distribution.archive_name(target), text)
                self.assertIn(hashlib.sha256(target.encode()).hexdigest(), text)
            self.assertIn('license "MIT"', text)
            self.assertIn('share/"memoria/skill/SKILL.md"', text)

    def test_incomplete_platform_set_cannot_emit_formula(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / distribution.archive_name("aarch64-apple-darwin")).write_bytes(b"archive")
            formula = root / "memoria.rb"
            with self.assertRaises(FileNotFoundError):
                distribution.formula(root, formula)
            self.assertFalse(formula.exists())

    def test_macos_rejects_machine_local_dependencies(self):
        result = SimpleNamespace(stdout="memoria:\n\t/opt/local/lib/libssl.3.dylib (compatibility version 3)\n")
        with patch.object(distribution, "run", return_value=result):
            with self.assertRaises(ValueError):
                distribution.check_linkage(Path("memoria"), "aarch64-apple-darwin")

    def test_macos_accepts_only_system_dependencies(self):
        result = SimpleNamespace(stdout="memoria:\n\t/usr/lib/libSystem.B.dylib (compatibility version 1)\n")
        with patch.object(distribution, "run", return_value=result):
            distribution.check_linkage(Path("memoria"), "aarch64-apple-darwin")

    def test_linux_rejects_unbundled_tls_and_missing_libraries(self):
        for library in ("libssl.so.3 => /lib/libssl.so.3 (0x123)", "libcrypto.so.3 => not found"):
            with self.subTest(library=library):
                with patch.object(distribution, "run", return_value=SimpleNamespace(stdout=library)):
                    with self.assertRaises(ValueError):
                        distribution.check_linkage(Path("memoria"), "x86_64-unknown-linux-gnu")


if __name__ == "__main__":
    unittest.main()
