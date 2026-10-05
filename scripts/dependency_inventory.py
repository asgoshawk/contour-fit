#!/usr/bin/env python3
"""Record locked dependency metadata and assemble third-party license texts."""
import argparse
import json
from pathlib import Path
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def license_sources(directory, declared):
    """Collect license text paths without allowing crate-controlled path escapes."""
    directory = directory.resolve()
    candidates = {p for pattern in ('LICENSE*', 'LICENCE*', 'COPYING*', 'license*', 'licenses/*')
                  for p in directory.glob(pattern) if not p.is_dir()}
    if declared:
        candidates.add(directory / declared)
    files = set()
    for candidate in candidates:
        resolved = candidate.resolve()
        if not resolved.is_relative_to(directory):
            raise ValueError('license source escapes the package directory')
        if resolved.is_file():
            files.add(resolved)
    return sorted(files)


def inventory(runtime_target=None):
    command = ['cargo', 'metadata', '--locked', '--format-version', '1']
    if runtime_target:
        command += ['--filter-platform', runtime_target]
    metadata = json.loads(subprocess.check_output(command, cwd=ROOT))
    lock = tomllib.loads((ROOT / 'Cargo.lock').read_text())
    checksums = {(p['name'], p['version']): p.get('checksum') for p in lock['package']}
    nodes = {n['id']: n for n in metadata['resolve']['nodes']}
    selected = {p['id'] for p in metadata['packages']}
    if runtime_target:
        root = next(p['id'] for p in metadata['packages'] if p['name'] == 'contour-fit')
        selected = set()
        pending = [root]
        while pending:
            package_id = pending.pop()
            if package_id in selected:
                continue
            selected.add(package_id)
            for dependency in nodes[package_id]['deps']:
                if any(kind['kind'] != 'dev' for kind in dependency['dep_kinds']):
                    pending.append(dependency['pkg'])
    packages = []
    notices = ['Third-party license texts from the locked dependency sources.\n' +
               ('This inventory includes target runtime and build dependencies.\n' if runtime_target else 'This inventory includes build and test dependencies.\n')]
    for package in sorted(metadata['packages'], key=lambda p: (p['name'], p['version'])):
        if package['id'] not in selected or package['id'] in metadata['workspace_members']:
            continue
        directory = Path(package['manifest_path']).parent.resolve()
        license_files = license_sources(directory, package.get('license_file'))
        packages.append({
            'name': package['name'], 'version': package['version'],
            'license': package['license'], 'repository': package['repository'],
            'source': package['source'],
            'checksum': checksums.get((package['name'], package['version'])),
            'build_script': any('custom-build' in t['kind'] for t in package['targets']),
            'procedural_macro': any('proc-macro' in t['kind'] for t in package['targets']),
            'license_files': [str(p.relative_to(directory)) for p in license_files],
            'dependencies': [{'package_id': d['pkg'], 'kinds': d['dep_kinds']}
                             for d in nodes[package['id']]['deps']
                             if d['pkg'] in selected and (not runtime_target or any(k['kind'] != 'dev' for k in d['dep_kinds']))],
        })
        notices.append(f"\n{'=' * 72}\n{package['name']} {package['version']} ({package['license']})\n")
        for file in license_files:
            notices.append(f'\n--- {file.relative_to(directory)} ---\n')
            notices.append(file.read_text(errors='replace') + '\n')
        if not license_files:
            notices.append('No standalone license file found; review package source before distribution.\n')
    return {'schema_version': 1, 'format': 'contour-fit dependency inventory',
            'target': runtime_target, 'packages': packages}, ''.join(notices)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runtime-target', help='Limit to CLI runtime/build dependencies for this target')
    parser.add_argument('--output', type=Path)
    parser.add_argument('--notices', type=Path)
    args = parser.parse_args()
    data, notices = inventory(args.runtime_target)
    text = json.dumps(data, indent=2, sort_keys=True) + '\n'
    if args.output:
        args.output.write_text(text)
    else:
        print(text, end='')
    if args.notices:
        missing = [p['name'] for p in data['packages'] if not p['license_files']]
        if missing and args.runtime_target:
            raise SystemExit('Cannot distribute without license texts: ' + ', '.join(missing))
        args.notices.write_text(notices)


if __name__ == '__main__':
    main()
