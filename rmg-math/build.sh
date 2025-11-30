#!/usr/bin/env bash
set -euo pipefail

pdflatex main.tex     # generates main.aux
bibtex main           # builds main.bbl from refs.bib
pdflatex main.tex     # incorporates citations
pdflatex main.tex     # final pass for cross-refs
