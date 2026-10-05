# Third-party notices

contour-fit is MIT licensed. Dependency code retains its own copyright and
license; this file does not replace those licenses or relicense dependencies.

Direct runtime dependencies:

- png — image-rs contributors, MIT OR Apache-2.0.
- kurbo — Kurbo/Linebender contributors, Apache-2.0 OR MIT.
- clap — clap contributors, MIT OR Apache-2.0.
- serde and serde_json — serde contributors, MIT OR Apache-2.0.
- thiserror — thiserror contributors, MIT OR Apache-2.0.
- resvg — resvg/Linebender contributors, Apache-2.0 OR MIT.

Development dependencies proptest and criterion are MIT OR Apache-2.0.
Full authoritative copyright and license texts are taken from the exact locked
crate sources. Before distributing a binary, generate and bundle
`DEPENDENCY_LICENSES.txt` and `dependencies.json` for that binary's target using
`scripts/dependency_inventory.py --runtime-target TARGET --notices ... --output ...`.
The Release candidate workflow performs this step. If license text collection
fails, do not distribute the artifact until resolved.

Numerical fitting follows the published Schneider method, implemented
independently; no Graphics Gems source was copied. Marching-squares connectivity
uses the standard algorithm with the documented four/eight-connectivity policy.
Reference links and selection rationale are in docs/architecture.md and
docs/dependencies.md. No upstream tracer source or user artwork is bundled.
