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

Run the commands in README before a PR. CI additionally validates Linux and
Apple Silicon. PRs changing visible SVG output should attach generated previews.
Dependency changes require license, sources, build-script/proc-macro, advisory,
and transitive-graph review; record exceptions with owner and expiry.

See [releases](docs/releases.md) for release/hotfix branches and compatibility.
For 0.x versions, incompatible public API changes require a minor version bump;
patches remain compatible. GitHub metadata and publishing require owner approval.
