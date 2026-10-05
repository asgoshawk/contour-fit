import unittest
import tempfile
from pathlib import Path
from unittest.mock import patch
from check_release import validate

class ReleasePolicyTests(unittest.TestCase):
    def test_alpha_branch_matches_version(self):
        validate('0.1.0-alpha.1', 'release/0.1.0')
        validate('0.1.0-alpha.1', 'hotfix/0.1.0')

    def test_rejects_develop_production_and_wrong_release_for_alpha(self):
        for branch in ('develop', 'production', 'release/0.2.0', 'feature/foo'):
            with self.assertRaises(ValueError):
                validate('0.1.0-alpha.1', branch)

    def test_invalid_semver_and_version_mismatch(self):
        for version in ('v0.1.0', '0.1', '0.1.0-alpha.0', '0.1.0-alpha.01', '0.1.0-beta.1', '01.1.0', '0.1.0-alpha.2'):
            with self.assertRaises(ValueError):
                validate(version, 'release/0.1.0')

    def test_official_version_requires_production(self):
        with self.assertRaisesRegex(ValueError, 'production'):
            validate('0.1.0', 'develop')
        with self.assertRaisesRegex(ValueError, 'workspace'):
            validate('0.1.0', 'production')

class OfficialReleaseFixtureTests(unittest.TestCase):
    def test_official_production_candidate_and_dependency_consistency(self):
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            cli = root / 'crates/contour-fit-cli'
            cli.mkdir(parents=True)
            (root / 'Cargo.toml').write_text('[workspace.package]\nversion = "0.1.0"\n')
            (cli / 'Cargo.toml').write_text('[dependencies.contour-fit-core]\nversion = "=0.1.0"\n')
            with patch('check_release.ROOT', root):
                validate('0.1.0', 'production')
                (cli / 'Cargo.toml').write_text('[dependencies.contour-fit-core]\nversion = "=0.1.0-alpha.1"\n')
                with self.assertRaisesRegex(ValueError, 'dependency'):
                    validate('0.1.0', 'production')

if __name__ == '__main__':
    unittest.main()
