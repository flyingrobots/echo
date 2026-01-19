#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
"""
Build the 'What Makes Echo Tick' tour document with:
1. Claude's commentary in red-outlined boxes with RED TEXT
2. PDF diagrams with embedded fonts
3. Letter-size paper with small margins
"""

import re
import subprocess
import sys
from pathlib import Path

STUDY_DIR = Path(__file__).parent
DIAGRAMS_DIR = STUDY_DIR / "diagrams"

INPUT_MD = STUDY_DIR / "what-makes-echo-tick.md"
PROCESSED_MD = STUDY_DIR / "what-makes-echo-tick-processed.md"
OUTPUT_TEX = STUDY_DIR / "what-makes-echo-tick.tex"
OUTPUT_PDF = STUDY_DIR / "what-makes-echo-tick.pdf"


def convert_commentary_to_latex(md_content: str) -> str:
    """Convert CLAUDE_COMMENTARY markers to LaTeX red boxes."""

    def replace_commentary(match):
        inner = match.group(1).strip()
        # Escape LaTeX special chars in the content
        # We'll handle this more carefully - just wrap in our environment
        return f'\n\n\\begin{{claudecommentary}}\n{inner}\n\\end{{claudecommentary}}\n\n'

    # Replace <!-- CLAUDE_COMMENTARY_START --> ... <!-- CLAUDE_COMMENTARY_END -->
    pattern = r'<!-- CLAUDE_COMMENTARY_START -->\s*(.*?)\s*<!-- CLAUDE_COMMENTARY_END -->'
    md_content = re.sub(pattern, replace_commentary, md_content, flags=re.DOTALL)

    return md_content


def convert_svg_to_pdf_refs(md_content: str) -> str:
    """Convert SVG image references to PDF for LaTeX."""
    md_content = re.sub(
        r'\!\[([^\]]*)\]\(diagrams/([^)]+)\.svg\)',
        r'![\1](diagrams/\2.pdf)',
        md_content
    )
    return md_content


def run_pandoc(md_file: Path, tex_file: Path) -> bool:
    """Run pandoc to convert markdown to LaTeX."""
    try:
        result = subprocess.run(
            [
                "pandoc",
                str(md_file),
                "-o", str(tex_file),
                "--standalone",
                "-f", "markdown+raw_tex",
                "--top-level-division=chapter",
                "-V", "geometry:margin=0.75in",
                "-V", "geometry:letterpaper",
                "-V", "fontsize=11pt",
            ],
            capture_output=True,
            text=True,
            timeout=60
        )
        if result.returncode != 0:
            print(f"pandoc failed: {result.stderr}", file=sys.stderr)
            return False
        return True
    except (subprocess.TimeoutExpired, FileNotFoundError) as e:
        print(f"pandoc error: {e}", file=sys.stderr)
        return False


def postprocess_tex(tex_file: Path) -> None:
    """Post-process the LaTeX file."""
    content = tex_file.read_text()

    # Add required packages and styling
    packages = r"""
\usepackage{graphicx}
\usepackage[export]{adjustbox}
\usepackage{tcolorbox}
\tcbuselibrary{breakable,skins}

% Page layout - small margins
\usepackage[margin=0.75in,letterpaper]{geometry}

% Make code blocks smaller to fit
\usepackage{fvextra}
\DefineVerbatimEnvironment{Highlighting}{Verbatim}{
    commandchars=\\\{\},
    fontsize=\small,
    breaklines=true,
    breakanywhere=true
}

% Define the Claude commentary box style - RED OUTLINE + RED TEXT
\newtcolorbox{claudecommentary}{
    enhanced,
    breakable,
    colback=red!5,
    colframe=red!75!black,
    coltext=red!70!black,
    boxrule=3pt,
    arc=5pt,
    left=12pt,
    right=12pt,
    top=12pt,
    bottom=12pt,
    before skip=15pt,
    after skip=15pt,
    fontupper=\color{red!70!black},
    fonttitle=\bfseries\Large\color{red!75!black},
    title={\raisebox{-0.1em}{\Large$\blacktriangleright$} Claude's Commentary},
    attach boxed title to top left={yshift=-4mm,xshift=10mm},
    boxed title style={
        colback=white,
        colframe=red!75!black,
        boxrule=2pt,
        arc=3pt
    }
}
"""

    # Insert packages after \documentclass
    if r'\usepackage{amsmath' in content:
        content = content.replace(
            r'\usepackage{amsmath',
            packages + r'\usepackage{amsmath'
        )
    elif r'\begin{document}' in content:
        content = content.replace(
            r'\begin{document}',
            packages + r'\begin{document}'
        )

    # Fix image includes - make them fit with max width/height
    content = re.sub(
        r'\\pandocbounded\{\\includegraphics\{([^}]+)\}\}',
        r'\\begin{center}\\includegraphics[max width=0.95\\textwidth,max height=0.4\\textheight,keepaspectratio]{\1}\\end{center}',
        content
    )

    # Also handle bare includegraphics
    content = re.sub(
        r'(?<!max width=0\.95\\textwidth,max height=0\.4\\textheight,keepaspectratio\]\{)\\includegraphics\{(diagrams/[^}]+)\}',
        r'\\begin{center}\\includegraphics[max width=0.95\\textwidth,max height=0.4\\textheight,keepaspectratio]{\1}\\end{center}',
        content
    )

    tex_file.write_text(content)


def run_xelatex(tex_file: Path) -> bool:
    """Run xelatex to produce PDF."""
    try:
        for run in [1, 2]:
            print(f"    xelatex pass {run}...", end=" ", flush=True)
            result = subprocess.run(
                [
                    "xelatex",
                    "-interaction=nonstopmode",
                    "-output-directory", str(tex_file.parent),
                    str(tex_file)
                ],
                capture_output=True,
                text=True,
                timeout=120,
                cwd=tex_file.parent
            )
            print("OK" if result.returncode == 0 else "warnings")

        return tex_file.with_suffix('.pdf').exists()

    except (subprocess.TimeoutExpired, FileNotFoundError) as e:
        print(f"xelatex error: {e}", file=sys.stderr)
        return False


def main():
    print("=== Building What Makes Echo Tick ===\n")

    if not INPUT_MD.exists():
        print(f"Error: {INPUT_MD} not found", file=sys.stderr)
        sys.exit(1)

    # Read the markdown
    print(f"1. Reading {INPUT_MD.name}...")
    md_content = INPUT_MD.read_text()

    # Convert commentary markers to LaTeX
    print("2. Converting Claude commentary to LaTeX red boxes...")
    md_content = convert_commentary_to_latex(md_content)

    # Convert SVG refs to PDF
    print("3. Converting image references to PDF...")
    md_content = convert_svg_to_pdf_refs(md_content)

    # Write processed markdown
    PROCESSED_MD.write_text(md_content)
    print(f"   Wrote {PROCESSED_MD.name}")

    # Run pandoc
    print("4. Running pandoc...")
    if not run_pandoc(PROCESSED_MD, OUTPUT_TEX):
        print("   Pandoc failed!")
        sys.exit(1)
    print(f"   Generated {OUTPUT_TEX.name}")

    # Post-process the LaTeX
    print("5. Post-processing LaTeX...")
    postprocess_tex(OUTPUT_TEX)
    print("   Added red boxes, small margins, fitted graphics")

    # Run xelatex
    print("6. Running xelatex...")
    if run_xelatex(OUTPUT_TEX):
        print(f"\n=== Success! ===")
        print(f"Output: {OUTPUT_PDF}")
    else:
        print("\n   PDF generation may have issues, check .log file")
        sys.exit(1)


if __name__ == "__main__":
    main()
