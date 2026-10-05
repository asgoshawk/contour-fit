# Security policy

This repository is an unpublished alpha. No released support window exists yet.
Please report a reproducible vulnerability through GitHub private vulnerability
reporting if enabled. Do not publish exploit details or attach private artwork
in public issues. If private reporting is unavailable, open a public issue that
asks the maintainer for a private contact without disclosing the vulnerability.

Runtime conversion is local. PNG decoding, contour work, fitting, and validation
have explicit limits; those limits are not a security sandbox. Avoid processing
untrusted images in a process with sensitive access. See docs/quality.md for
allocation limits and numerical guarantees.

Before distribution, run cargo-deny against a freshly updated RustSec database.
An unavailable database means verification is incomplete. Do not suppress new
advisories silently. Any exception must have a reason, owner, and expiry.
All new git dependencies are denied; registry sources are restricted to crates.io.
Third-party unsafe/build scripts are reviewed separately from the first-party
unsafe prohibition. Read docs/dependencies.md for the recorded dependency review.

Release workflows only build reviewable artifacts with read-only repository
permissions. Publishing is a separate, owner-approved operation.
