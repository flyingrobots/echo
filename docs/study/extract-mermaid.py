#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
"""
Extract Mermaid diagrams from Markdown files and convert to PDF via SVG.

Pipeline: .md -> extract mermaid blocks -> .mmd -> mmdc -> .svg -> inkscape -> .pdf
"""

import re
import subprocess
import sys
from pathlib import Path

STUDY_DIR = Path(__file__).parent
DIAGRAMS_DIR = STUDY_DIR / "diagrams"

def extract_mermaid_blocks(md_file: Path) -> list[tuple[str, str]]:
    """Extract mermaid code blocks from a markdown file.

    Returns list of (diagram_id, mermaid_code) tuples.
    """
    content = md_file.read_text()

    # Match ```mermaid ... ``` blocks
    pattern = r'```mermaid\n(.*?)```'
    matches = re.findall(pattern, content, re.DOTALL)

    results = []
    base_name = md_file.stem

    for i, code in enumerate(matches, 1):
        diagram_id = f"{base_name}-{i:02d}"
        results.append((diagram_id, code.strip()))

    return results


def convert_mermaid_to_pdf(diagram_id: str, mermaid_code: str, output_dir: Path) -> Path | None:
    """Convert mermaid code to PDF via SVG.

    Returns path to PDF or None on failure.
    """
    output_dir.mkdir(parents=True, exist_ok=True)

    mmd_file = output_dir / f"{diagram_id}.mmd"
    svg_file = output_dir / f"{diagram_id}.svg"
    pdf_file = output_dir / f"{diagram_id}.pdf"

    # Write mermaid source
    mmd_file.write_text(mermaid_code)

    # Convert to SVG with mmdc
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

    if not svg_file.exists():
        print(f"  SVG not created for {diagram_id}", file=sys.stderr)
        return None

    # Convert SVG to PDF with inkscape
    try:
        result = subprocess.run(
            ["inkscape", str(svg_file), "--export-type=pdf", f"--export-filename={pdf_file}"],
            capture_output=True,
            text=True,
            timeout=30
        )
        if result.returncode != 0:
            print(f"  inkscape failed for {diagram_id}: {result.stderr}", file=sys.stderr)
            return None
    except subprocess.TimeoutExpired:
        print(f"  inkscape timeout for {diagram_id}", file=sys.stderr)
        return None
    except FileNotFoundError:
        print("  inkscape not found", file=sys.stderr)
        return None

    if pdf_file.exists():
        return pdf_file
    return None


def main():
    """Process all markdown files in study directory."""
    md_files = [
        STUDY_DIR / "what-makes-echo-tick.md",
        STUDY_DIR / "echo-visual-atlas.md",
        STUDY_DIR / "echo-tour-de-code.md",
    ]

    total_diagrams = 0
    converted = 0

    for md_file in md_files:
        if not md_file.exists():
            print(f"Skipping {md_file.name} (not found)")
            continue

        print(f"\n=== Processing {md_file.name} ===")
        blocks = extract_mermaid_blocks(md_file)
        print(f"Found {len(blocks)} mermaid diagrams")

        for diagram_id, code in blocks:
            total_diagrams += 1
            print(f"  Converting {diagram_id}...", end=" ")

            pdf_path = convert_mermaid_to_pdf(diagram_id, code, DIAGRAMS_DIR)
            if pdf_path:
                print(f"OK -> {pdf_path.name}")
                converted += 1
            else:
                print("FAILED")

    print(f"\n=== Summary ===")
    print(f"Total diagrams: {total_diagrams}")
    print(f"Converted: {converted}")
    print(f"Failed: {total_diagrams - converted}")
    print(f"Output directory: {DIAGRAMS_DIR}")


if __name__ == "__main__":
    main()
