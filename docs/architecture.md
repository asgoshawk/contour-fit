# Architecture

The workspace has two crates. `contour-fit-core` owns numerical geometry and
`contour-fit` owns the CLI and adapters. Public core types do not expose kurbo,
png, resvg, or serialization types.

```mermaid
flowchart LR
  PNG[PNG decoder] --> Field[RasterField]
  Field --> Contour[Marching squares and topology]
  Contour --> Samples[Arc-length samples and protected corners]
  Samples --> Fit[Bounded cubic fitting and splitting]
  Fit --> Verify[Independent geometry and topology validator]
  Verify --> Merge[Validated adjacent merging]
  Merge --> Round[SVG coordinate rounding]
  Round --> VerifyExport[Revalidate exported geometry]
  VerifyExport --> SVG[SVG and JSON]
  VerifyExport --> Debug[Optional PNG diagnostics]
```

## Core responsibilities

- `geometry`: project-owned points, line/cubic curves, subdivision and distances.
- `contour`: grid-edge stitching, containment, deterministic winding/start points,
  and arc-length resampling.
- `fit`: protected corner detection, endpoint-tangent least squares, bounded
  Newton reparameterization, and iterative adaptive splitting.
- `metrics`: a separately implemented bidirectional distance/BVH validator,
  conservative sample coverage, and topology checking.
- `optimize`: deterministic adjacent merges accepted only after global validation.
- `pipeline`: budgets and ordering, including one denser retry on quality failure.

`vectorize(&RasterField, &FitOptions)` returns `Vectorization` or `FitError`.
`validate_paths` lets adapters certify rounded curves against returned references.
All coordinates and tolerances are source pixels; arithmetic uses f64.

## Fitting decisions

Marching squares operates on the unbinarized scalar field. Pixel centers are
(x + 0.5, y + 0.5); a virtual zero-valued frame closes boundary-touching contours.
Foreground uses four-connectivity and background eight-connectivity in saddle
cells. Threshold ties are nudged only at crossing endpoints by 1e-9 grid units.
Contours sort by descending absolute area and start at the topmost, then leftmost
vertex. Parents precede children and depth determines winding.

Uniform fitting samples use spacing min(tolerance / 2, 1 px), augmented with exact
protected source vertices. Turning angles on the source contours are
probed at 1 and 2 px, with deterministic nonmaximum suppression; the default
protected-corner angle is 60 degrees. The canonical seam is never merged. Source junctions with the image boundary are
also protected; control arm lengths are constrained to the canvas along their
tangent directions so SVG clipping cannot invalidate the geometric check.

The initial fitter uses chord-length parameterization, a guarded 2x2 least-squares
solve for tangent-aligned control arms, and at most five Newton refinement passes.
Invalid or nonmonotonic parameters are rejected. Recursive splitting is replaced
by an explicit stack. Shared smooth tangents provide G1 joins; corner tangents
are independent. A quality failure triggers one denser/stricter retry, never a
relaxed user tolerance. Initial splitting targets 35% of the user tolerance to
reserve validation headroom. Merge candidates can use the full tolerance because
each acceptance passes independent global validation. At most 128 adjacent merge attempts are made globally.
No optimal segment-count claim is made.

## Adapters and output

The CLI expands palette/low-bit-depth PNG and preserves 16-bit precision. It
rejects APNG, oversized decoded buffers, corrupted images, and output aliases.
Dark luminance is foreground; encoded sample values use Rec.709 weights without
ICC/gamma color management. Explicit luminance multiplies darkness by alpha.
Auto selects alpha whenever any pixel is nonopaque. This is predictable mask
processing, not photo segmentation.

The exporter rounds to 3 through 9 decimal places, validating each candidate.
It emits a single evenodd/currentColor path and keeps the canvas. Output files
are reserved with create_new only after validation; handled I/O failures roll
back files created by that invocation. This prevents overwrites but is not a
cross-file atomic transaction against power loss or concurrent hostile changes.

## Rejected first-version alternatives

- LM and centripetal initialization: not benchmarked in this repository yet.
  Keep the smaller chord-length/least-squares baseline until measurements justify
  adding another solver or initializer; superiority is not established.
- kurbo's area/moment fitter: useful comparison, but sampled approximate Fréchet
  error is not a substitute for independent validation.
- Potrace integration: native integration and GPL distribution obligations.
- VTracer integration: broad tracing policy and older image/CLI dependencies;
  compare independently rather than importing its full pipeline.
- Parallelism/SIMD: profile first; neither is required for current single-logo use.

Sources: [Schneider implementation](https://github.com/erich666/GraphicsGems/blob/master/gems/FitCurves.c),
[Raph Levien's analysis](https://raphlinus.github.io/curves/2021/03/11/bezier-fitting.html),
[kurbo 0.13.1](https://github.com/linebender/kurbo/blob/v0.13.1/kurbo/src/fit.rs),
[marching-squares documentation](https://scikit-image.org/docs/stable/api/skimage.measure.html#skimage.measure.find_contours).
The Schneider method was implemented independently; no Graphics Gems code was copied.
