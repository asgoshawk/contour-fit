#!/usr/bin/env python3
"""Validate a release candidate without creating tags or publishing anything."""
import argparse
from pathlib import Path
import re
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def validate(version: str, branch: str) -> None:
    match = re.fullmatch(r'(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-alpha\.([1-9]\d*))?', version)
    if not match:
        raise ValueError('expected X.Y.Z or X.Y.Z-alpha.N (N starts at 1)')
    base = '.'.join(match.group(i) for i in (1, 2, 3))
    if match.group(4):
        if branch not in (f'release/{base}', f'hotfix/{base}'):
            raise ValueError('alpha candidates must come from the matching release or hotfix branch')
    elif branch != 'production':
        raise ValueError('official candidates must come from production')
    workspace = tomllib.loads((ROOT / 'Cargo.toml').read_text())
    cli = tomllib.loads((ROOT / 'crates/contour-fit-cli/Cargo.toml').read_text())
    if workspace['workspace']['package']['version'] != version:
        raise ValueError('workspace version differs from the requested release')
    if cli['dependencies']['contour-fit-core']['version'] != f'={version}':
        raise ValueError('CLI core dependency version differs from the workspace')


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('version')
    parser.add_argument('--branch', required=True)
    args = parser.parse_args()
    try:
        validate(args.version, args.branch)
    except ValueError as error:
        print(f'release check: {error}', file=sys.stderr)
        return 1
    print(f'Validated {args.version} on {args.branch}; no tag or release was created.')
    return 0


if __name__ == '__main__':
    sys.exit(main())
