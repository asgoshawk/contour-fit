# Performance snapshot

Measured on 2026-10-05 with Rust 1.97.0, release thin-LTO, on a shared macOS ARM64 host.
Host: `macOS-26.6.2-arm64-arm-64bit-Mach-O`. The exact CPU model was unavailable in the sandbox.
These are generated fixtures and local measurements, not a promise for arbitrary artwork.

## End-to-end CLI

One new process per case; PNG creation is excluded. Includes PNG decoding, fitting,
export revalidation, JSON/SVG writing, and optional debug rendering/PNG output.
Child peak RSS is recorded using getrusage (bytes on macOS). Commands ran after the
core benchmark completed; normal system background activity was not controlled.

| Shape | Size | Debug | Wall time (ms) | Peak RSS (MiB) | Segments | Max bound (px) |
| --- | ---: | --- | ---: | ---: | ---: | ---: |
| circle | 512 | no | 409.73 | 6.48 | 4 | 0.2623 |
| scalloped | 512 | no | 70.19 | 6.31 | 18 | 0.8694 |
| circle | 1024 | no | 24.21 | 13.44 | 4 | 0.3212 |
| scalloped | 1024 | no | 276.23 | 16.30 | 25 | 0.9513 |
| circle | 2048 | no | 51.55 | 39.72 | 4 | 0.3935 |
| scalloped | 2048 | no | 614.65 | 42.03 | 63 | 0.9794 |
| scalloped | 1024 | yes | 261.59 | 20.88 | 25 | 0.9513 |

The 1024 px scalloped fixture satisfies the initial 2-second / 256-MiB local target.
The debug run records native-resolution IoU of 0.998498.
These single-process samples are distinct from Criterion repeated sampling below.

## Core-only Criterion measurements

10 samples, 1-second warm-up, 2-second requested measurement period (Criterion
extends collection for slow cases). Input scalar fields are already allocated;
PNG, exporter revalidation, I/O and diagnostics are excluded.

| Fixture | 512 px | 1024 px | 2048 px |
| --- | ---: | ---: | ---: |
| circle | 3.598 ms | 9.087 ms | 22.606 ms |
| scalloped | 67.170 ms | 208.093 ms | 572.258 ms |

The first CLI case took 409.73 ms in this run. Earlier first-process observations
also varied substantially, sometimes while other work ran concurrently; the
cause was not isolated. Treat these single wall-time observations as illustrative,
not a stable latency distribution. The Criterion rows report mean point estimates.

Compared with the stricter merge prefilter, validated full-budget merging reduced
this 1024 px scalloped PNG from 45 to 25 segments. The core scalloped fixture took
about 208 ms instead of 87 ms in successive local runs, consistent with more
candidates reaching global validation. No profiler isolated that cause. This
trades time for fewer segments within the same
1 px tolerance; `--no-merge` skips that optional work. The fixtures differ in
scalar precision (PNG alpha quantized to 8 bits versus the core f32 field), so
core and CLI timings are not directly comparable.
No parallel fitting, GPU, or system image-processing dependency is used.

## Reproduce

```sh
cargo build --release --locked
cargo bench -p contour-fit-core --bench pipeline --locked -- --warm-up-time 1 --measurement-time 2
python3 scripts/measure_cli.py --output target/new-measurement-directory
```

Use a new output directory each time. Avoid concurrent compilation/benchmarking
when comparing results. Fixtures are analytic alpha circles and nine-lobed
scalloped outlines, created in code without third-party artwork.

Rust source snapshot SHA-256 (including tests and benches):
`35ba878d7f4812d05724c0b7b48ec38be3ccdb6c148af5edb27361e50c3fdb29`.

Linux tests and release builds passed in [the initial PR's CI](https://github.com/asgoshawk/contour-fit/actions/runs/37322464650).
The timing and memory measurements above remain macOS-only. A separate
[user-provided bird demo](assets/demo/README.md) records one real-image conversion;
it does not establish performance or quality across other artwork.
Raw generated images, reports and Criterion samples live under target/ and are not committed.
