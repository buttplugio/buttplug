#!/usr/bin/env bash
# Ensures all .rs files in crates/ have the proper license header.
# Usage: ./scripts/check_headers.sh [--fix]
#   Without --fix: reports files with missing/wrong headers (exit 1 if any found)
#   With --fix:    adds or replaces headers in-place

set -euo pipefail

HEADER='// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.'

FIX=false
if [[ "${1:-}" == "--fix" ]]; then
  FIX=true
fi

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATES_DIR="$REPO_ROOT/crates"

missing=()
wrong=()

while IFS= read -r -d '' file; do
  first_line=$(head -1 "$file")

  if [[ "$first_line" == "// Buttplug Rust Source Code File"* ]]; then
    # Has a Buttplug header — check if it matches exactly
    existing=$(head -7 "$file")
    if [[ "$existing" != "$HEADER" ]]; then
      wrong+=("$file")
      if $FIX; then
        # Strip old header (first 7 lines) and prepend correct one
        tail -n +8 "$file" > "$file.tmp"
        printf '%s\n' "$HEADER" > "$file"
        cat "$file.tmp" >> "$file"
        rm "$file.tmp"
      fi
    fi
  else
    missing+=("$file")
    if $FIX; then
      # Prepend header + blank line
      cp "$file" "$file.tmp"
      printf '%s\n\n' "$HEADER" > "$file"
      cat "$file.tmp" >> "$file"
      rm "$file.tmp"
    fi
  fi
done < <(find "$CRATES_DIR" -name '*.rs' -print0)

exit_code=0

if [[ ${#wrong[@]} -gt 0 ]]; then
  if $FIX; then
    echo "Fixed header in ${#wrong[@]} file(s) with wrong header:"
  else
    echo "Wrong header in ${#wrong[@]} file(s):"
  fi
  for f in "${wrong[@]}"; do
    echo "  ${f#$REPO_ROOT/}"
  done
  exit_code=1
fi

if [[ ${#missing[@]} -gt 0 ]]; then
  if $FIX; then
    echo "Added header to ${#missing[@]} file(s) missing header:"
  else
    echo "Missing header in ${#missing[@]} file(s):"
  fi
  for f in "${missing[@]}"; do
    echo "  ${f#$REPO_ROOT/}"
  done
  exit_code=1
fi

if [[ $exit_code -eq 0 ]]; then
  echo "All .rs files have the correct header."
fi

if $FIX; then
  exit 0
fi
exit $exit_code
