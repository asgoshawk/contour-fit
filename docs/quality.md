# Quality contract and numerical limits

The reference is the continuous polyline extracted by marching squares at the
selected scalar threshold. It is not a guarantee of recovering an original
vector drawing or the true shape depicted in a photograph.

## Geometric error

- RMS and P95 are estimated source-to-output distances at uniform arc-length
  samples; tiny rings get at least eight samples. They are diagnostic, not gates.
- `sampled_hausdorff_px` is the maximum of both sampled directed distances to
  the other polyline. It is not exact continuous Hausdorff distance.
- Cubics are subdivided until both interior control points are within epsilon of
  their endpoint segment. The convex-hull property bounds curve-to-chord error;
  continuity of projection covers chord-to-curve error as well.
- Both polylines are sampled with arc-length spacing at most tolerance / 8.
  Every point is within spacing / 2 of a sample. Add that coverage radius, the
  output flattening epsilon, and a 1e-8 px numerical margin to the sampled maximum.
  This is `hausdorff_upper_bound_px`, the hard gate.
- The initial flattening epsilon is tolerance / 32. Ambiguous topology reduces
  epsilon by four, up to seven checks; exhausted precision fails explicitly.

The bound is conservative geometric reasoning under f64 arithmetic, not an
interval-arithmetic/formal proof. Raster dimensions, tolerances, and finite-value
checks bound the numerical operating domain. Do not market it as exact Hausdorff
or exact topological certification. SVG serialization is checked with the same
contract after rounding actual written coordinates. G1 tangents can acquire tiny
angular differences at that serialization precision.

## Topology

Closed paths, nonzero area, winding, containment parent, self-intersections,
inter-ring intersections, and ambiguous near contacts are checked on bounded
subdivision polylines. Nonadjacent edges require separation greater than twice
the flattening allowance. Adjacent edges necessarily share a vertex and are
excluded from that separation test. Features below the reported checking
resolution are not a formal topological guarantee. No automatic hole deletion,
morphological cleanup, or cropping is performed.

## Raster diagnostics

Debug mode renders the exported SVG at its original canvas resolution using
resvg. IoU compares source values >= the requested threshold with rendered alpha
>= 128/255, using native-resolution pixels (no supersampling). The report records
both thresholds and resolution. Empty union is defined as IoU 1. IoU is not a
hard gate; one-pixel features make fixed IoU thresholds misleading.

## Budgets and failures

Defaults: 16,777,216 pixels; 250,000 extracted/resampled points; 20,000 curves;
500,000 samples/flattened edges; 100,000 fitting attempts and a separate 100,000
topology work-unit budget (bounding-box comparisons and contained polygon edges). PNG
buffers and decoder allocations are separately limited to 128 MiB. These are
per-stage work/allocation bounds, not a process-wide resident-memory cap.
Tolerances are limited to 0.001..=64 px. Tight tolerances or highly noisy input
can exhaust budgets; failure is preferable to silent detail loss.

Exit codes: 0 success; 2 invalid input/options or no foreground; 3 quality
failure; 4 I/O/resource/reporting failure. No SVG is written on quality failure.

## Reproducibility

Contour sorting, seams, split ties, and merge order are deterministic. The same
platform/toolchain/input/options produce the same SVG. Reports contain measured
timings, so JSON is not byte-for-byte reproducible. Across platforms compare
geometry and the documented numerical limits instead of binary output identity.
