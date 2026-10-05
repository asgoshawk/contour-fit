# contour-fit

Deterministic silhouette-to-SVG fitting with error-controlled Bézier curves,
topology preservation, and a Rust library and CLI.

**Development version: `0.1.0-alpha.1` (not published).** Supports transparent
and black-on-white silhouette PNGs, independent components, and nested holes.
Photos, segmentation, animation, and color vectorization are outside v0.1.

[繁體中文](docs/zh-TW/README.md) · [Architecture](docs/architecture.md) ·
[Quality definitions](docs/quality.md) · [Releases](docs/releases.md) ·
[Supply-chain review](docs/dependencies.md) · [Performance](docs/performance.md)

## Build and use

Install the pinned Rust toolchain with rustup, then:

```sh
cargo build --release --locked
./target/release/contour-fit silhouette.png -o silhouette.svg \
  --report silhouette.metrics.json --debug-dir silhouette-debug
```

All output destinations must be new. Parent directories must already exist;
only the optional debug directory is created. The tool never overwrites inputs.
Runtime conversion does not access the network.

```sh
contour-fit image.png -o image.svg --channel alpha --threshold 0.5
contour-fit white-on-black.png -o image.svg --channel luminance --invert
contour-fit image.png -o image.svg --tolerance 0.25 --no-merge
```

The default tolerance is **1 source pixel**, measured against the marching-squares
level-set contours, not unknown pre-rasterization geometry. Alpha is selected
when nonopaque pixels exist; otherwise auto selects dark luminance as foreground.
Cleanup and cropping are never performed automatically.

Output is one `currentColor` path with closed subpaths and `evenodd` fill. SVG
coordinate rounding is revalidated before writing. Debug mode produces a PNG
preview, an overlay (red reference, blue fitted contours), and raster IoU in the
optional JSON report. Timings vary; SVG geometry is deterministic on the same
platform and toolchain. [See numerical limits and metric definitions](docs/quality.md).

## Library

```rust
use contour_fit_core::{FitError, FitOptions, RasterField, vectorize};

fn main() -> Result<(), FitError> {
    let field = RasterField::new(2, 2, vec![1.0, 0.0, 0.0, 0.0])?;
    let result = vectorize(&field, &FitOptions::default())?;
    assert!(result.report.hausdorff_upper_bound_px <= 1.0);
    Ok(())
}
```

The API returns closed paths, parent/depth metadata, unmodified reference contours,
and a quality report. It performs no file I/O, PNG decoding, or SVG rendering.
API changes before 1.0 may occur in minor releases; patch releases stay compatible.

## Development

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
python3 -m unittest discover -s scripts -p 'test_*.py'
cargo build --workspace --release --locked
cargo bench -p contour-fit-core --bench pipeline --locked
cargo install cargo-deny --version 0.20.2 --locked
cargo deny --locked check advisories licenses bans sources
```

Use **feature → develop → release/X.Y.Z → production**, with alpha versions
on the matching release branch and official versions on production. The initial
production bootstrap is not a release. See [CONTRIBUTING](CONTRIBUTING.md) for TDD
and [release policy](docs/releases.md) for hotfixes, approvals, and immutable tags.
No workflow automatically publishes GitHub releases or crates.io packages.
CI uses standard Linux and Apple Silicon runners for this public repository;
candidate packaging is manual with 7-day artifact retention.
[GitHub Free scope and storage limits](docs/releases.md#github-free-and-ci-scope).

## License

[MIT](LICENSE). Third-party notices are recorded in
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md); distribution artifacts include
full dependency license texts. Input artwork and generated outputs do not
inherit this project's software license.
