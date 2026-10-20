#!/bin/bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
R=/private/tmp/claude-501/-Users-j-git-echo/2df23d48-982e-4dec-bd5a-3de7c779ebaf/scratchpad/runners
$R/checks.sh /Users/j/git/echo-claude-base base suite
$R/checks.sh /Users/j/git/echo-claude-inline inline suite
$R/checks.sh /Users/j/git/echo-claude-commitpath commitpath suite
