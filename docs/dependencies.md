# Dependency selection and supply-chain review

Snapshot: 2026-10-05; workspace version 0.1.0-alpha.1. Cargo.lock pins 108 external
packages including development and non-host platform dependencies. The macOS
ARM64 CLI runtime/build graph contains 61 external packages. Counts are a
snapshot, not an acceptance threshold.

## Direct dependencies

| Package | Pin | License | Purpose / decision |
| --- | --- | --- | --- |
| png | 0.18.1 | MIT OR Apache-2.0 | Narrow PNG input/output; preserve 16-bit samples |
| kurbo | 0.13.1 | Apache-2.0 OR MIT | Geometry; std only, no optional interop/schema features |
| clap | 4.6.7 | MIT OR Apache-2.0 | Typed argument parser |
| serde / serde_json | 1.0.229 / 1.0.151 | MIT OR Apache-2.0 | Versioned diagnostic JSON |
| thiserror | 2.0.21 | MIT OR Apache-2.0 | Typed errors |
| resvg | 0.48.1 | Apache-2.0 OR MIT | Render only self-generated SVG; defaults disabled |
| proptest | 1.11.0 | MIT OR Apache-2.0 | Development-only property testing; std only |
| criterion | 0.8.2 | Apache-2.0 OR MIT | Development-only benchmarks; default extras disabled |

Candidate crate ownership/repositories, published metadata, licenses, and direct
and transitive dependencies were inspected. `cargo-deny 0.20.2 --locked check
advisories licenses bans sources` passed against the downloaded RustSec database.
No advisory exception is configured. A clean advisory scan does not prove that
the packages are safe or exhaustively audited.

## Build and unsafe surface

The recorded host runtime/build inventory identifies compiler/configuration
scripts in crc32fast, num-traits, proc-macro2, quote, serde, serde_core,
serde_json, thiserror, and zmij; procedural macros in clap_derive, serde_derive,
and thiserror-impl. Review focused on compiler probes and generated source/config
in build scripts, package provenance, and explicit features; it is not a full
manual audit of every dependency source line. Inventory retains build-script and
proc-macro markers so upgrades cannot hide that execution surface.

First-party code uses `forbid(unsafe_code)` and workspace lint enforcement.
Third-party crates can use unsafe code, including numerical containers, CRC/SIMD
kernels, rasterization, and JSON performance paths. Do not describe this as an
entirely unsafe-free graph. Criterion transitively introduces alloca/cc and a
small native C build for development benchmarks; those packages are not in the
shipped CLI runtime graph. Neither OpenCV nor a system image library is required.

The only reviewed duplicate warning is miniz_oxide 0.8.9 (png) and 0.9.1
(flate2). Both satisfy the current advisory policy; the duplication remains
visible rather than hidden behind a blanket skip. Changing upstream compression
versions solely to remove this warning would add an unnecessary compatibility
choice. Review again on dependency updates.

## Licensing and distribution

`deny.toml` restricts source registries and acceptable licenses. Dual/multiple
SPDX expressions are evaluated by cargo-deny; MIT/Apache options are selected
where available, not an LGPL alternative. Unknown registries and Git dependencies
are denied. All direct packages are pinned; full transitive checksums are in
Cargo.lock and checked by Cargo.

Generate an all-target inventory for review:

```sh
python3 scripts/dependency_inventory.py --output target/dependencies.json
```

Generate distribution notices from the actual target runtime/build graph:

```sh
python3 scripts/dependency_inventory.py \
  --runtime-target aarch64-apple-darwin \
  --output target/runtime-dependencies.json \
  --notices target/DEPENDENCY_LICENSES.txt
```

Distribution fails if a selected package has no collectable license text.
Non-shipped test/Windows/UEFI packages in the complete inventory can lack
standalone license files; do not accidentally redistribute that broader source
bundle using runtime-only notices. Current host distribution inventory has no
missing license texts. This JSON is a dependency inventory, not an SPDX/CycloneDX
SBOM or a vulnerability attestation.

## Update and CI policy

Review dependency PRs for advisory state, license/provenance, checksums, optional
feature changes, build scripts, procedural macros, and new unsafe/native code.
No secret-bearing environment is needed. Re-run tests, release builds, audits,
and relevant numerical benchmarks when dependencies affect behavior.

CI installs pinned cargo-deny and uses `--locked`. GitHub Actions are pinned to
full upstream commit IDs. Audit database update failures fail the job; cached or
unavailable data must not be reported as current verification. Any future
exception needs a documented reason, owner, and expiration.

Sources: [crates.io](https://crates.io/data-access),
[RustSec](https://github.com/RustSec/advisory-db),
[cargo-deny](https://embarkstudios.github.io/cargo-deny/),
[resvg feature manifest](https://github.com/linebender/resvg/blob/v0.48.1/crates/resvg/Cargo.toml),
[Potrace license](https://potrace.sourceforge.net/#license),
[VTracer dependencies](https://crates.io/api/v1/crates/vtracer/0.6.5/dependencies).
