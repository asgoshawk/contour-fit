# Bird demo

This silhouette was supplied by the user on 2026-10-05 and included with their
permission for use in the repository README. The software's MIT license does
not grant a separate license to the artwork. No separate artwork license has
been specified.

## Files

| File | Role |
| --- | --- |
| [bird-source.png](bird-source.png) | Original 1254 × 1254 RGBA input, unchanged. |
| [bird.svg](bird.svg) | Exact CLI output; transparent, with the original canvas. |
| [bird-report.json](bird-report.json) | Recorded quality, raster overlap, and timings from the original run. |
| [bird-comparison.svg](bird-comparison.svg) | README figure, embedding the PNG and the exported SVG paths on white panels. |
| [bird-comparison.png](bird-comparison.png) | Browser-rendered 1000 × 600 preview used in Markdown for consistent image display. |

The comparison figure adds labels and display backgrounds. It does not alter
the input PNG or the downloadable SVG geometry. All 3 extracted contours are
retained, with 62 curve segments in total; no cleanup was applied.

## Reproduce the conversion

From the repository root, with the release binary already built:

```sh
mkdir -p artifacts
./target/release/contour-fit docs/assets/demo/bird-source.png \
  -o artifacts/bird.svg \
  --report artifacts/bird-report.json \
  --debug-dir artifacts/bird-debug
```

Output files and the debug directory must be new. The recorded run used
`0.1.0-alpha.1`, auto channel selection (alpha for this PNG), threshold 0.5,
1 px tolerance, 60-degree corner detection, and merging enabled. Export uses
3 decimal places. It passed the independent geometric and topology checks.

The report measures distance from the extracted threshold contour; it does not
recover a pre-rasterization drawing. IoU is measured at 1254 × 1254, using source
alpha >= 0.5 and rendered alpha >= 128/255. Timing fields vary between runs.
[Quality definitions](../../quality.md).

After replacing demo assets, rebuild the comparison SVG without extra packages:

```sh
python3 scripts/build_demo.py
```

Render that figure to a 1000 × 600 PNG in a browser and update
`bird-comparison.png` too. The current preview was rendered in headless Chrome;
no raster retouching was applied. Keep the full figure visible, including both
panels and labels, and check that the exported paths still match `bird.svg`.
