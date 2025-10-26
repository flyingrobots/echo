#!/usr/bin/env bash
set -euo pipefail

if ! command -v gum >/dev/null 2>&1; then
  echo "gum is required but not installed." >&2
  exit 1
fi

ROOT_DIR=$(pwd)
DEFAULT_PROJECT_NAME=${PROJECT_NAME:-$(basename "$ROOT_DIR")}
DEFAULT_DESCRIPTION=$(sed -n '1s/^# //p;1q' README.md 2>/dev/null || echo "")
DEFAULT_PACKAGE_MANAGER=${PACKAGE_MANAGER:-cargo}
DEFAULT_ARCH_DOC=${ARCHITECTURE_DOC:-docs/architecture-outline.md}
DEFAULT_EXEC_PLAN=${EXECUTION_PLAN_DOC:-docs/execution-plan.md}
DEFAULT_AGENT_GUIDE=${AGENT_GUIDE:-AGENTS.md}
DEFAULT_TAG=${PROJECT_TAG:-Echo}
DEFAULT_DEVLOG=${DEVLOG_THREAD:-echo-devlog}
DEFAULT_SPEC_THREAD=${SPEC_THREAD:-echo-spec}
DEFAULT_YEAR=$(date +%Y)
DEFAULT_COPYRIGHT=${COPYRIGHT_HOLDER:-"${DEFAULT_PROJECT_NAME} Contributors"}
DEFAULT_LEGACY_DIR=${LEGACY_DIR:-docs/legacy/}
DEFAULT_BRANCH=${DEFAULT_BRANCH:-main}
DEFAULT_CODEOWNER=${CODEOWNER_HANDLE:-@flyingrobots}
DEFAULT_SECURITY_EMAIL=${SECURITY_EMAIL:-security@${DEFAULT_PROJECT_NAME,,}.dev}

PROJECT_NAME=$(gum input --prompt "Project name" --value "$DEFAULT_PROJECT_NAME")
DESCRIPTION=$(gum input --prompt "Project description" --value "$DEFAULT_DESCRIPTION")
PACKAGE_MANAGER=$(gum input --prompt "Package manager command" --value "$DEFAULT_PACKAGE_MANAGER")
ARCHITECTURE_DOC=$(gum input --prompt "Architecture doc path" --value "$DEFAULT_ARCH_DOC")
EXECUTION_PLAN_DOC=$(gum input --prompt "Execution plan doc" --value "$DEFAULT_EXEC_PLAN")
AGENT_GUIDE=$(gum input --prompt "Agent onboarding doc" --value "$DEFAULT_AGENT_GUIDE")
PROJECT_TAG=$(gum input --prompt "Project tag for timeline notes" --value "$DEFAULT_TAG")
DEVLOG_THREAD=$(gum input --prompt "Devlog document or label" --value "$DEFAULT_DEVLOG")
SPEC_THREAD=$(gum input --prompt "Spec document or label" --value "$DEFAULT_SPEC_THREAD")
YEAR=$(gum input --prompt "Copyright year" --value "$DEFAULT_YEAR")
COPYRIGHT_HOLDER=$(gum input --prompt "Copyright holder" --value "$DEFAULT_COPYRIGHT")
LEGACY_DIR=$(gum input --prompt "Legacy directory reference" --value "$DEFAULT_LEGACY_DIR")
DEFAULT_BRANCH=$(gum input --prompt "Default branch" --value "$DEFAULT_BRANCH")
CODEOWNER_HANDLE=$(gum input --prompt "CODEOWNER handle" --value "$DEFAULT_CODEOWNER")
SECURITY_EMAIL=$(gum input --prompt "Security contact email" --value "$DEFAULT_SECURITY_EMAIL")

LICENSE_TEMPLATE_DEFAULT=$(gum input --prompt "License template (default MIT)" --value "MIT")
if [[ "$LICENSE_TEMPLATE_DEFAULT" == "MIT" ]]; then
  LICENSE_TEXT=$(cat <<'EOM'
MIT License

Copyright (c) ${YEAR} ${COPYRIGHT_HOLDER}

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
EOM
)
else
  LICENSE_TEXT=$(gum input --prompt "Paste license text" --placeholder "License body" --value "${LICENSE_TEMPLATE_DEFAULT}")
fi

export PROJECT_NAME
export DESCRIPTION
export PACKAGE_MANAGER
export ARCHITECTURE_DOC
export EXECUTION_PLAN_DOC
export AGENT_GUIDE
export PROJECT_TAG
export DEVLOG_THREAD
export SPEC_THREAD
export YEAR
export COPYRIGHT_HOLDER
export LEGACY_DIR
export DEFAULT_BRANCH
export CODEOWNER_HANDLE
export SECURITY_EMAIL
export LICENSE_TEXT

OUTPUTS=("CONTRIBUTING.md" "LICENSE" "NOTICE" "SECURITY.md" "CODEOWNERS")
SELECTION=$(printf '%s
' "${OUTPUTS[@]}" | gum choose --no-limit)
if [[ -z "$SELECTION" ]]; then
  echo "No files selected; exiting."
  exit 0
fi

declare -A TEMPLATE_MAP
TEMPLATE_MAP[CONTRIBUTING.md]="templates/CONTRIBUTING.md.tmpl"
TEMPLATE_MAP[LICENSE]="templates/LICENSE.tmpl"
TEMPLATE_MAP[NOTICE]="templates/NOTICE.tmpl"
TEMPLATE_MAP[SECURITY.md]="templates/SECURITY.md.tmpl"
TEMPLATE_MAP[CODEOWNERS]="templates/CODEOWNERS.tmpl"

for TARGET in $SELECTION; do
  TEMPLATE=${TEMPLATE_MAP[$TARGET]}
  if [[ ! -f "$TEMPLATE" ]]; then
    echo "Template $TEMPLATE missing" >&2
    continue
  fi
  DEST="$ROOT_DIR/$TARGET"
  if [[ -f "$DEST" ]]; then
    if ! gum confirm "${TARGET} exists. Overwrite?"; then
      echo "Skipped $TARGET"
      continue
    fi
  fi
  envsubst < "$TEMPLATE" > "$DEST"
  echo "Wrote $TARGET"

done

echo
echo "Done. Generated files:" $SELECTION
