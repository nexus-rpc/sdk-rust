#!/usr/bin/env bash
set -euo pipefail

version="$1"
if ! notes="$(awk -v heading="[$version]" '
  /^## / {
    if (section) exit
    section = ($2 == heading)
    next
  }
  section && (content || NF) {
    print
    content = 1
  }
  END { if (!content) exit 1 }
' CHANGELOG.md)"; then
  echo "CHANGELOG.md has no release notes for $version." >&2
  exit 1
fi
printf '## Notable Changes\n\n%s\n' "$notes"
