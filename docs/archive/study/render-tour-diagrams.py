#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
"""
Extract mermaid blocks from what-makes-echo-tick-tour.md,
render them to SVG, and update the markdown to reference the SVGs.
"""

import re
import subprocess
import sys
from pathlib import Path

STUDY_DIR = Path(__file__).parent
DIAGRAMS_DIR = STUDY_DIR / "tour-diagrams"
INPUT_MD = STUDY_DIR / "what-makes-echo-tick-tour.md"


def extract_mermaid_blocks(content: str) -> list[tuple[int, int, str]]:
    """Extract (start, end, code) tuples for all mermaid blocks."""
    pattern = r'```mermaid\n(.*?)```'
    results = []
    for match in re.finditer(pattern, content, re.DOTALL):
        results.append((match.start(), match.end(), match.group(1).strip()))
    return results


def render_mermaid_to_svg(diagram_id: str, mermaid_code: str) -> Path | None:
    """Render mermaid code to SVG. Returns path to SVG or None on failure."""
    DIAGRAMS_DIR.mkdir(parents=True, exist_ok=True)

    mmd_file = DIAGRAMS_DIR / f"{diagram_id}.mmd"
    svg_file = DIAGRAMS_DIR / f"{diagram_id}.svg"

    mmd_file.write_text(mermaid_code)

    try:
        result = subprocess.run(
            ["mmdc", "-i", str(mmd_file), "-o", str(svg_file), "-b", "transparent"],
            capture_output=True,
            text=True,
            timeout=30
        )
        if result.returncode != 0:
            print(f"  mmdc failed for {diagram_id}: {result.stderr}", file=sys.stderr)
            return None
    except subprocess.TimeoutExpired:
        print(f"  mmdc timeout for {diagram_id}", file=sys.stderr)
        return None
    except FileNotFoundError:
        print("  mmdc not found - install with: npm install -g @mermaid-js/mermaid-cli", file=sys.stderr)
        return None

    if svg_file.exists():
        return svg_file
    return None


def main():
    print("=== Rendering Tour Diagrams ===\n")

    content = INPUT_MD.read_text()
    blocks = extract_mermaid_blocks(content)

    print(f"Found {len(blocks)} mermaid diagrams")

    # Process in reverse order to preserve string positions
    for i, (start, end, code) in enumerate(reversed(blocks), 1):
        diagram_num = len(blocks) - i + 1
        diagram_id = f"tour-{diagram_num:02d}"

        print(f"  Converting {diagram_id}...", end=" ")

        svg_path = render_mermaid_to_svg(diagram_id, code)
        if svg_path:
            # Replace mermaid block with image reference
            # Use relative path from study dir
            img_ref = f"![Diagram {diagram_num}](tour-diagrams/{diagram_id}.svg)"
            content = content[:start] + img_ref + content[end:]
            print("OK")
        else:
            print("FAILED")

    # Write updated markdown
    INPUT_MD.write_text(content)
    print(f"\nUpdated {INPUT_MD.name} with SVG references")
    print(f"Diagrams saved to {DIAGRAMS_DIR}")


if __name__ == "__main__":
    main()
