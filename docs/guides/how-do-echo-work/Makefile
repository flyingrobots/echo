# SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
# Makefile for How Echo Works LaTeX Guide

.PHONY: all clean view

LATEX_ENGINE ?= pdflatex
TEX_FILE = main.tex
PDF_FILE = $(TEX_FILE:.tex=.pdf)

all: $(PDF_FILE)

$(PDF_FILE): $(TEX_FILE)
	@echo "Building LaTeX document..."
	$(LATEX_ENGINE) -shell-escape $(TEX_FILE)
	$(LATEX_ENGINE) -shell-escape $(TEX_FILE)
	$(LATEX_ENGINE) -shell-escape $(TEX_FILE)
	@echo "LaTeX build complete: $(PDF_FILE)"

clean:
	@echo "Cleaning up generated LaTeX files..."
	rm -f *.aux *.log *.toc *.lof *.lot *.fls *.synctex.gz *.xdv *.ps *.dvi *.out *.nav *.snm *.vrb
	rm -f $(PDF_FILE)
	@echo "Cleanup complete."

view: all
	@echo "Opening PDF..."
	$(eval OS := $(shell uname -s))
	$(ifeq ($(OS),Darwin), open $(PDF_FILE),)
	$(ifeq ($(OS),Linux), xdg-open $(PDF_FILE),)
	$(ifeq ($(OS),Windows), start $(PDF_FILE),)
