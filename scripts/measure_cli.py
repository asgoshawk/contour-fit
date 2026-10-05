#!/usr/bin/env python3
"""Generate synthetic masks and measure release CLI wall time and child peak RSS."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import platform
import resource
import struct
import subprocess
import sys
import time
import zlib

ROOT = Path(__file__).resolve().parents[1]


def chunk(kind, data):
    return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))


def fixture(size, complex_shape):
    mid = size / 2
    rows = bytearray()
    for y in range(size):
        row = bytearray(size * 4)
        for x in range(size):
            dx, dy = x + .5 - mid, y + .5 - mid
            radius = mid * (.65 + .08 * math.cos(9 * math.atan2(dy, dx))) if complex_shape else mid * .7
            value = min(1, max(0, radius - math.hypot(dx, dy) + .5))
            row[x * 4 + 3] = round(255 * value)
        rows.append(0)
        rows.extend(row)
    return (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', size, size, 8, 6, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress(rows)) + chunk(b'IEND', b''))


def measure(directory, binary, size, shape, debug):
    directory.mkdir()
    source, output, report = (directory / name for name in ('input.png', 'output.svg', 'metrics.json'))
    source.write_bytes(fixture(size, shape == 'scalloped'))
    command = [str(binary), str(source), '-o', str(output), '--report', str(report)]
    if debug:
        command += ['--debug-dir', str(directory / 'debug')]
    start = time.perf_counter()
    process = subprocess.run(command, capture_output=True, text=True)
    wall = time.perf_counter() - start
    if process.returncode:
        raise RuntimeError(process.stderr)
    rss = resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss
    if sys.platform != 'darwin':
        rss *= 1024
    data = json.loads(report.read_text())
    return {'size_px': size, 'shape': shape, 'debug': debug, 'wall_ms': wall * 1000,
            'peak_rss_mib': rss / 1048576, 'quality': data['quality'],
            'timings_ms': data['timings_ms'], 'raster': data['raster']}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, default=ROOT / 'target/release/contour-fit')
    parser.add_argument('--output', type=Path, default=ROOT / 'target/cli-measurements')
    parser.add_argument('--worker', action='store_true', help=argparse.SUPPRESS)
    parser.add_argument('--size', type=int, default=1024, help=argparse.SUPPRESS)
    parser.add_argument('--shape', default='circle', help=argparse.SUPPRESS)
    parser.add_argument('--debug', action='store_true', help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.worker:
        print(json.dumps(measure(args.output, args.binary.resolve(), args.size, args.shape, args.debug)))
        return
    args.output.mkdir()
    measurements = []
    cases = [(size, shape, False) for size in (512, 1024, 2048) for shape in ('circle', 'scalloped')]
    cases.append((1024, 'scalloped', True))
    for size, shape, debug in cases:
        directory = args.output / f'{shape}-{size}{"-debug" if debug else ""}'
        command = [sys.executable, str(Path(__file__).resolve()), '--worker', '--binary', str(args.binary),
                   '--output', str(directory), '--size', str(size), '--shape', shape]
        if debug:
            command.append('--debug')
        measurement = json.loads(subprocess.check_output(command, text=True))
        measurements.append(measurement)
        print(f'{shape} {size} debug={debug}: {measurement["wall_ms"]:.2f} ms, {measurement["peak_rss_mib"]:.2f} MiB')
    digest = hashlib.sha256()
    for source in sorted((ROOT / 'crates').rglob('*.rs')):
        digest.update(str(source.relative_to(ROOT)).encode())
        digest.update(source.read_bytes())
    result = {'platform': platform.platform(), 'architecture': platform.machine(),
              'rust_source_sha256': digest.hexdigest(), 'method': 'one fresh CLI process per case; PNG creation excluded',
              'measurements': measurements}
    (args.output / 'summary.json').write_text(json.dumps(result, indent=2) + '\n')


if __name__ == '__main__':
    main()
