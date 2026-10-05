import tempfile
from pathlib import Path
import unittest
from dependency_inventory import license_sources

class LicenseCollectionTests(unittest.TestCase):
    def test_collects_source_texts_without_duplicates(self):
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            (root / 'LICENSE-MIT').write_text('test license')
            self.assertEqual(license_sources(root, 'LICENSE-MIT'), [(root / 'LICENSE-MIT').resolve()])

    def test_rejects_paths_outside_the_crate(self):
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch) / 'crate'
            root.mkdir()
            (root.parent / 'outside').write_text('must not be read')
            for declared in ('../outside', str(root.parent / 'outside')):
                with self.assertRaises(ValueError):
                    license_sources(root, declared)

    def test_rejects_symlinked_license_escape(self):
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch) / 'crate'
            root.mkdir()
            (root.parent / 'outside').write_text('must not be read')
            (root / 'LICENSE').symlink_to(root.parent / 'outside')
            with self.assertRaises(ValueError):
                license_sources(root, None)

if __name__ == '__main__':
    unittest.main()
