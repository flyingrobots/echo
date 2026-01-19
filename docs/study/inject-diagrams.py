#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
"""
Post-process LaTeX files to replace mermaid code blocks with diagram includes.

Finds Shaded blocks containing mermaid syntax and replaces with \includegraphics.
"""

import re
import sys
from pathlib import Path

STUDY_DIR = Path(__file__).parent
DIAGRAMS_DIR = STUDY_DIR / "diagrams"

# Mermaid start patterns
MERMAID_STARTS = [
    r'\\NormalTok\{graph ',
    r'\\NormalTok\{flowchart ',
    r'\\NormalTok\{sequenceDiagram\}',
    r'\\NormalTok\{classDiagram\}',
    r'\\NormalTok\{stateDiagram',
    r'\\NormalTok\{erDiagram\}',
    r'\\NormalTok\{pie ',
    r'\\NormalTok\{gantt\}',
]


def is_mermaid_block(block_content: str) -> bool:
    """Check if a Shaded block contains mermaid diagram syntax."""
    for pattern in MERMAID_STARTS:
        if re.search(pattern, block_content):
            return True
    return False


def process_tex_file(tex_file: Path, base_name: str) -> str:
    """Process a tex file, replacing mermaid blocks with includegraphics."""
    content = tex_file.read_text()

    # Match Shaded environments
    shaded_pattern = r'\\begin\{Shaded\}(.*?)\\end\{Shaded\}'

    diagram_counter = 0
    replacements = []

    for match in re.finditer(shaded_pattern, content, re.DOTALL):
        block = match.group(0)
        block_content = match.group(1)

        if is_mermaid_block(block_content):
            diagram_counter += 1
            diagram_id = f"{base_name}-{diagram_counter:02d}"
            pdf_path = DIAGRAMS_DIR / f"{diagram_id}.pdf"

            if pdf_path.exists():
                # Create centered figure with the diagram
                replacement = (
                    f"\\begin{{center}}\n"
                    f"\\includegraphics[max width=\\textwidth,max height=0.4\\textheight,keepaspectratio]"
                    f"{{diagrams/{diagram_id}.pdf}}\n"
                    f"\\end{{center}}"
                )
                replacements.append((match.start(), match.end(), replacement))
            else:
                print(f"  Warning: {pdf_path.name} not found, keeping code block")

    # Apply replacements in reverse order to preserve positions
    for start, end, replacement in reversed(replacements):
        content = content[:start] + replacement + content[end:]

    # Add graphicx package if we made replacements and it's not already there
    if replacements and r'\usepackage{graphicx}' not in content:
        # Insert after documentclass or after other usepackage statements
        content = content.replace(
            r'\usepackage{longtable',
            r'\usepackage{graphicx}' + '\n' + r'\usepackage[export]{adjustbox}' + '\n' + r'\usepackage{longtable'
        )

    return content, len(replacements)


def main():
    """Process all tex files."""
    tex_files = [
        ("what-makes-echo-tick.tex", "what-makes-echo-tick"),
        ("echo-visual-atlas.tex", "echo-visual-atlas"),
        ("echo-tour-de-code.tex", "echo-tour-de-code"),
    ]

    for tex_name, base_name in tex_files:
        tex_file = STUDY_DIR / tex_name
        if not tex_file.exists():
            print(f"Skipping {tex_name} (not found)")
            continue

        print(f"\n=== Processing {tex_name} ===")
        new_content, count = process_tex_file(tex_file, base_name)

        if count > 0:
            # Write to new file (preserve original)
            output_file = STUDY_DIR / tex_name.replace('.tex', '-with-diagrams.tex')
            output_file.write_text(new_content)
            print(f"  Replaced {count} mermaid blocks")
            print(f"  Output: {output_file.name}")
        else:
            print(f"  No mermaid blocks found")


if __name__ == "__main__":
    main()
