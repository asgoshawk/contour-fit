# Branch and release policy

The default branch is `develop`. `production` contains accepted releases; its
initial documentation/license bootstrap is not a release. `main` is not used.

| Branch | Starts from | Integrates into | Policy |
| --- | --- | --- | --- |
| feature/*, fix/* | develop | develop | PR, passing checks, squash merge |
| develop | bootstrap | release/X.Y.Z | Continuous integration, no published alpha tags |
| release/X.Y.Z | develop | production and develop | Freeze scope; only fixes, docs, version changes; merge commits |
| production | bootstrap | — | Official tags only; no direct development |
| hotfix/X.Y.Z | latest official tag | production and develop | Patch fixes; backport into active release branches |

## First release

1. Cut `release/0.1.0` from validated develop.
2. Keep workspace and CLI core-dependency versions equal to `0.1.0-alpha.1`.
3. Run `python3 scripts/check_release.py 0.1.0-alpha.1 --branch release/0.1.0`.
4. Run all CI checks and benchmark the candidate. The manually dispatched Release
   candidate workflow produces binaries, checksums, source SHA, dependency
   metadata, and full license texts. It creates no tag or public release.
5. After explicit owner approval, create immutable `v0.1.0-alpha.1` at that exact
   reviewed commit and publish a GitHub prerelease. Subsequent candidates use
   alpha.2, alpha.3, etc.; update both manifests and Cargo.lock each time.
6. When acceptance and security gates pass, change the version to `0.1.0`, freeze
   changes, and run checks again. Merge into production with a merge commit.
7. Verify the production merge, build the exact merged commit, and run
   `python3 scripts/check_release.py 0.1.0 --branch production`. Formal release
   artifacts must identify this production SHA, not an earlier alpha build.
8. After explicit owner approval, create `v0.1.0` on production and publish a
   non-prerelease GitHub release. crates.io publication needs separate approval.
9. Merge release changes back to develop, delete the short-lived release branch,
   and prepare the next minor alpha development version.

Never move or overwrite a tag. Any correction receives a new version. Do not
squash release/hotfix integration; preserve release ancestry. Hotfix alphas, if
needed, use the matching hotfix/X.Y.Z branch before the official patch.

## Acceptance gates

- Required formatting, Clippy, tests, release build, release-policy tests,
  current dependency audit, and version consistency pass.
- Test geometry satisfies its tolerance and topology checks. Known alpha
  limitations are documented; official versions cannot have unexplained failures.
- Representative 512/1024/2048 px benchmarks and memory measurements are recorded.
  The initial 1024 px target is <=2 seconds and <=256 MiB on Apple Silicon;
  it must be measured, not inferred from architecture.
- No unapproved security exception; all dependency notices are included.
- 0.x minor releases may change the public API, patches remain compatible.
  Version 0.1.0 does not promise the stable API contract of 1.0.0.

## Remote setup (owner-approved, not performed by local development)

Set description to the README sentence and default branch to develop. Protect
both develop and production with PR-only merges, required conversation
resolution, CI checks `quality (ubuntu-24.04)` and `quality (macos-15)`,
and no force pushes or deletion. The Linux quality job includes the dependency
audit. Protect release and hotfix
branches similarly. Squash merges are for feature PRs; select merge commits for
release/hotfix PRs. Do not enable automatic publication.

References: [SemVer](https://semver.org/),
[GitHub protected branches](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches).

Reviewable REST payloads are checked in as `.github/repository-settings.json`
and `.github/branch-protection.json`. They are **not automatically applied**.
The protection payload permits solo-maintainer PRs (zero required approving
reviews) while requiring passing CI and resolved conversations. It protects
admins too. Apply it to each existing long-lived/release/hotfix branch after
owner approval; future wildcard protection needs a separately reviewed ruleset.

## GitHub Free and CI scope

The repository was confirmed public through the GitHub REST API on 2026-10-05.
The account's subscription and billing settings were not inspected. This setup
targets GitHub Free and uses only standard `ubuntu-24.04` and Apple Silicon
`macos-15` runners. Public-repository standard runner minutes are free; public
branch protection is available on Free. Larger runners are excluded because
they are billed even for public repositories.

Keep automation to two workflows:

- **CI:** PRs into develop/production/release/hotfix, pushes to develop and
  production, or manual runs. Two platform jobs run formatting, linting, tests,
  and a release build; Linux also audits dependencies. All-target Clippy checks
  benchmark code; timing benchmarks remain manual to avoid noisy CI thresholds.
  CI uploads no artifacts and uses no persistent cache.
- **Release candidate:** manual run against an explicit branch and version;
  validates and packages Linux/Apple Silicon binaries, checksums, source commit,
  and runtime license inventory. Artifacts expire after 7 days. Publishing and
  tagging remain explicit owner actions.

Both workflows use read-only repository permissions and cancel obsolete runs.
There is no deployment service, paid action, merge queue, attestation service,
or automatic registry publishing in this initial setup.

Free currently includes 500 MB of artifact storage shared with GitHub Packages;
short retention reduces usage but does not guarantee zero storage charges.
Keep artifact/Packages usage within that allowance, remove stale candidates,
and leave paid overage disabled (or use an Actions budget with
**Stop usage when budget limit is reached** enabled) in account
billing if a strict zero-cost policy is required. These account settings cannot
be enforced by this repository's YAML and were not changed.

If made private later, reevaluate before enabling workflows: Free's included
2,000 monthly runner minutes and storage limits apply, and Free does not offer
private-repository branch protection. Do not silently assume public-repo terms
continue to apply.

Sources checked 2026-10-05:
[repository visibility](https://api.github.com/repos/asgoshawk/contour-fit),
[Actions billing and allowances](https://docs.github.com/en/billing/concepts/product-billing/github-actions),
[standard runners](https://docs.github.com/en/actions/reference/runners/github-hosted-runners),
 [budget enforcement](https://docs.github.com/en/billing/how-tos/set-up-budgets),
and [Free branch protection availability](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches).
