# SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
# Makefile for How Echo Works LaTeX Guide

.PHONY: all clean view booklets master logos

LATEX_ENGINE ?= pdflatex
LATEX_FLAGS = -interaction=nonstopmode -halt-on-error -file-line-error

SVG_LOGOS := ../../assets/ECHO.svg ../../assets/ECHO_chunky.svg
PDF_LOGOS := graphics/ECHO.pdf graphics/ECHO_chunky.pdf

MASTER_TEX := main.tex
MASTER_PDF := main.pdf

BOOKLETS_TEX := booklet-01-orientation.tex booklet-02-core.tex booklet-03-math.tex booklet-04-ops.tex
BOOKLETS_PDF := $(BOOKLETS_TEX:.tex=.pdf)

all: master booklets

master: $(PDF_LOGOS) $(MASTER_PDF)

booklets: $(PDF_LOGOS) $(BOOKLETS_PDF)

$(PDF_LOGOS): $(SVG_LOGOS)
	@echo "Converting logos to PDF..."
	rsvg-convert -f pdf -o graphics/ECHO.pdf ../../assets/ECHO.svg
	rsvg-convert -f pdf -o graphics/ECHO_chunky.pdf ../../assets/ECHO_chunky.svg

$(MASTER_PDF): $(MASTER_TEX)
	@echo "Building master volume..."
	$(LATEX_ENGINE) $(LATEX_FLAGS) $(MASTER_TEX)
	$(LATEX_ENGINE) $(LATEX_FLAGS) $(MASTER_TEX)
	$(LATEX_ENGINE) $(LATEX_FLAGS) $(MASTER_TEX)

%.pdf: %.tex
	@echo "Building $@..."
	$(LATEX_ENGINE) $(LATEX_FLAGS) $<
	$(LATEX_ENGINE) $(LATEX_FLAGS) $<
	$(LATEX_ENGINE) $(LATEX_FLAGS) $<

clean:
	@echo "Cleaning up generated LaTeX files..."
	rm -f *.aux *.log *.toc *.lof *.lot *.fls *.synctex.gz *.xdv *.ps *.dvi *.out *.nav *.snm *.vrb
	rm -f $(MASTER_PDF) $(BOOKLETS_PDF)
	@echo "Cleanup complete."

view: master
	@echo "Opening PDF..."
	$(eval OS := $(shell uname -s))
	$(ifeq ($(OS),Darwin), open $(MASTER_PDF),)
	$(ifeq ($(OS),Linux), xdg-open $(MASTER_PDF),)
	$(ifeq ($(OS),Windows), start $(MASTER_PDF),)
