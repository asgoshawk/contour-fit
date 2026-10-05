# Contributing

Target `develop` with small, reviewable PRs. Use English code, comments, commit
messages, and primary documentation; maintain the linked zh-TW summaries when
behavior changes. Use Conventional Commits. Do not commit target/, artifacts,
local configuration, input artwork without permission, or credentials.

## TDD and Clean Code

1. Describe an observable behavior and add a regression or property test.
2. Run that test and confirm the intended failure.
3. Implement the smallest coherent change.
4. Run the focused tests, refactor responsibilities and names, then run required checks.

Test contracts, numerical edge cases, and failure modes; do not merely reproduce
the fitting implementation in a test. Keep the geometry validator independent
of the fitter. Protect finite inputs, budgets, precision, corner semantics, and
closed-path invariants. Avoid global mutable state, speculative plugin layers,
large functions mixing I/O with math, and first-party unsafe code.

## Check your changes

Run these from the repository root before a code PR:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
python3 -m unittest discover -s scripts -p 'test_*.py'
cargo build --workspace --release --locked
cargo install cargo-deny --version 0.20.2 --locked
cargo deny --locked check advisories licenses bans sources
```

For changes to fitting or validation, also measure the relevant cases with
`cargo bench -p contour-fit-core --bench pipeline --locked`. See
[performance](docs/performance.md) for the measurement method.

For documentation-only changes, check links, commands, image rendering, and
preservation of factual claims. If demo assets change, rerun the demonstrated
conversion and compare its geometry and quality report. Rebuild the comparison
SVG with `python3 scripts/build_demo.py`, then refresh its PNG preview as
described in the [demo notes](docs/assets/demo/README.md).

CI validates Linux and Apple Silicon. PRs changing visible SVG output should
attach generated previews.
Dependency changes require license, sources, build-script/proc-macro, advisory,
and transitive-graph review; record exceptions with owner and expiry.

See [releases](docs/releases.md) for release/hotfix branches and compatibility.
For 0.x versions, incompatible public API changes require a minor version bump;
patches remain compatible. GitHub metadata and publishing require owner approval.
