#!/usr/bin/env python3
"""Rebuild the comparison SVG from demo assets; render its PNG preview separately."""
import base64
import json
from pathlib import Path
import struct
import xml.etree.ElementTree as ET

DEMO = Path(__file__).resolve().parents[1] / 'docs/assets/demo'
SVG_NS = 'http://www.w3.org/2000/svg'
ET.register_namespace('', SVG_NS)


def build():
    source = (DEMO / 'bird-source.png').read_bytes()
    svg_bytes = (DEMO / 'bird.svg').read_bytes()
    report = json.loads((DEMO / 'bird-report.json').read_text())
    width, height = struct.unpack('>II', source[16:24])
    svg = ET.fromstring(svg_bytes)
    svg.set('x', '544')
    svg.set('y', '96')
    svg.set('width', '408')
    svg.set('height', '408')
    nested_svg = ET.tostring(svg, encoding='unicode')
    encoded_source = base64.b64encode(source).decode('ascii')
    comparison = f'''<svg xmlns="{SVG_NS}" width="1000" height="600" viewBox="0 0 1000 600" role="img" aria-labelledby="title description">
  <title id="title">PNG silhouette to fitted SVG</title>
  <desc id="description">The supplied bird PNG on the left and its fitted SVG on the right. White panels make the black silhouettes visible in both light and dark themes.</desc>
  <rect width="1000" height="600" fill="#f8fafc"/>
  <g fill="#ffffff" stroke="#dbe2ea">
    <rect x="24" y="24" width="456" height="552" rx="12"/>
    <rect x="520" y="24" width="456" height="552" rx="12"/>
  </g>
  <g font-family="Arial, Helvetica, sans-serif" fill="#111827">
    <g font-size="20" font-weight="600">
      <text x="48" y="60">PNG input</text>
      <text x="544" y="60">Fitted SVG</text>
    </g>
    <g font-size="13" fill="#475569">
      <text x="48" y="86">{width} × {height} · transparent silhouette</text>
      <text x="544" y="86">Same canvas · scalable paths</text>
      <text x="48" y="548">Original raster</text>
      <text x="544" y="548">{report['quality']['segments']} segments · {len(svg_bytes) / 1024:.2f} KiB</text>
    </g>
  </g>
  <image x="48" y="96" width="408" height="408" href="data:image/png;base64,{encoded_source}"/>
  <g color="#000000">{nested_svg}</g>
  <path d="M 488 300 H 510 M 502 292 L 510 300 L 502 308" fill="none" stroke="#64748b" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
</svg>
'''
    (DEMO / 'bird-comparison.svg').write_text(comparison)


if __name__ == '__main__':
    build()
