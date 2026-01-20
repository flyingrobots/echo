% SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0
% © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
\NeedsTeXFormat{LaTeX2e}
\ProvidesClass{aion}[2025/12/07 AIΩN Foundations Series Class]

\LoadClass[11pt]{article}

% ------------------------------------------------------------
% Packages
% ------------------------------------------------------------
\RequirePackage[T1]{fontenc}
\RequirePackage{lmodern}
\RequirePackage{amsmath, amssymb, amsfonts, amsthm}
\RequirePackage{microtype}
\RequirePackage{geometry}
\RequirePackage{xcolor}
\RequirePackage{graphicx}
\RequirePackage{titlesec}
\RequirePackage{tocloft}
\RequirePackage{enumitem}
\RequirePackage{booktabs}
\RequirePackage{chngcntr}
\RequirePackage{hyperref}

% ------------------------------------------------------------
% Geometry
% ------------------------------------------------------------
\geometry{
    margin=1in,
}

% ------------------------------------------------------------
% Colors (brand)
% ------------------------------------------------------------
\definecolor{AIONBlue}{RGB}{20, 60, 120}
\definecolor{AIONAccent}{RGB}{120, 20, 120}

% ------------------------------------------------------------
% Hyperref
% ------------------------------------------------------------
\hypersetup{
    colorlinks=true,
    linkcolor=AIONBlue,
    citecolor=AIONBlue,
    urlcolor=AIONAccent,
    hypertexnames=false
}

% ------------------------------------------------------------
% Section Titles
% ------------------------------------------------------------
\titleformat{\section}
  {\large\bfseries\color{AIONBlue!85!black}}
  {\thesection}{0.5em}{}

\titleformat{\subsection}
  {\normalsize\bfseries\color{AIONBlue!85!black}}
  {\thesubsection}{0.5em}{}

% Reset figure numbering by section for cleaner hyperlinks
\counterwithin{figure}{section}
\renewcommand{\theHfigure}{\thesection.\arabic{figure}}

% ------------------------------------------------------------
% Theorem Environments
% ------------------------------------------------------------
\theoremstyle{definition}
\newtheorem{definition}{Definition}[section]
\newtheorem{assumption}[definition]{Assumption}

\theoremstyle{plain}
\newtheorem{proposition}[definition]{Proposition}
\newtheorem{theorem}[definition]{Theorem}
\newtheorem{lemma}[definition]{Lemma}
\newtheorem{corollary}[definition]{Corollary}

\theoremstyle{remark}
\newtheorem{example}[definition]{Example}
\newtheorem{remark}[definition]{Remark}

% ------------------------------------------------------------
% Metadata Commands
% ------------------------------------------------------------
% Internal storage macros (initialized to \@empty for robust checking)
\makeatletter
\newcommand{\AION@papertitle}{\@empty}
\newcommand{\AION@papernumber}{\@empty}
\newcommand{\AION@paperversion}{\@empty}
\newcommand{\AION@paperdate}{\@empty}
\newcommand{\AION@paperauthor}{\@empty}
\newcommand{\AION@paperaffiliation}{\@empty}
\newcommand{\AION@paperorcid}{\@empty}
\newcommand{\AION@paperdoi}{\@empty}

% User-facing setter commands
\newcommand{\papertitle}[1]{\gdef\AION@papertitle{#1}}
\newcommand{\papernumber}[1]{\gdef\AION@papernumber{#1}}
\newcommand{\paperversion}[1]{\gdef\AION@paperversion{#1}}
\newcommand{\paperdate}[1]{\gdef\AION@paperdate{#1}}
\newcommand{\paperauthor}[1]{\gdef\AION@paperauthor{#1}}
\newcommand{\paperaffiliation}[1]{\gdef\AION@paperaffiliation{#1}}
\newcommand{\paperorcid}[1]{\gdef\AION@paperorcid{#1}}
\newcommand{\paperdoi}[1]{\gdef\AION@paperdoi{#1}}

% Robust emptiness check using \@empty
\newcommand{\AION@require}[2]{%
  \ifx#1\@empty
    \ClassError{aion}{#2 not set}{You must call #2 before \string\AIONTitlePage}
  \fi
}
\makeatother

\makeatletter
\newcommand{\AIONTitlePage}{%
    \AION@require{\AION@papertitle}{\string\papertitle}%
    \AION@require{\AION@papernumber}{\string\papernumber}%
    \AION@require{\AION@paperauthor}{\string\paperauthor}%
    \AION@require{\AION@paperdate}{\string\paperdate}%

    \thispagestyle{empty}
    \begin{center}

        % Nudge the block slightly downward for visual gravity
        \vspace*{1.5cm}

        % Title (primary anchor)
        {\Huge\bfseries \AION@papertitle \par}
        \vspace{14pt}

        % Series / paper number (subtitle energy)
        {\normalsize\scshape\color{AIONBlue}
        AI$\Omega$N Foundations Series — \AION@papernumber \par}

        \vspace{16pt}

        % Author block (confident, quiet)
        {\large
        \AION@paperauthor \par}
        \vspace{4pt}

        % Only show affiliation/ORCID if defined
        \ifx\AION@paperaffiliation\@empty\else
        {\normalsize \AION@paperaffiliation \par}
        \fi
        \ifx\AION@paperorcid\@empty\else
        {\normalsize ORCID: \AION@paperorcid \par}
        \fi
        \ifx\AION@paperdoi\@empty\else
        {\normalsize DOI: \AION@paperdoi \par}
        \fi

        \vspace{10pt}

        {\normalsize
        \AION@paperdate \par}

    \end{center}
    \vspace{2cm}
}
\makeatother

\newcommand{\AIONFrontMatter}[1]{%
    \begin{center}
        \small
        #1
    \end{center}
    \vspace{1em}
}

% ------------------------------------------------------------
% Table of Contents formatting
% TODO: implement custom TOC styling if needed
% ------------------------------------------------------------

\endinput
