#!/usr/bin/env bash
set -euo pipefail

metadata="$(cargo metadata --format-version 1 --no-deps)"
publishable="$(jq -r '
  .packages[] | select(.publish == null or (.publish | index("crates-io")))
  | [.name, .version] | @tsv
' <<<"$metadata")"
index_file="$(mktemp)"
trap 'rm -f "$index_file"' EXIT

unpublished=()
while IFS=$'\t' read -r package version; do
  [[ -n "$package" ]] || continue
  name="${package,,}"
  case "${#name}" in
    1|2) path="${#name}/$name" ;;
    3) path="3/${name:0:1}/$name" ;;
    *) path="${name:0:2}/${name:2:2}/$name" ;;
  esac

  status="$(curl --silent --show-error --max-time 30 \
    --user-agent 'nexus-rpc/sdk-rust release planner (https://github.com/nexus-rpc/sdk-rust)' \
    --output "$index_file" --write-out '%{http_code}' "https://index.crates.io/$path")"
  case "$status" in
    200)
      present="$(jq -s --arg version "$version" 'any(.[]; .vers == $version)' "$index_file")"
      if [[ "$present" == true ]]; then
        echo "$package@$version is already published; skipping." >&2
      else
        unpublished+=("$package")
      fi
      ;;
    404) unpublished+=("$package") ;;
    *)
      echo "Failed to check $package@$version: sparse index returned HTTP $status." >&2
      exit 1
      ;;
  esac
done <<<"$publishable"

plan="$(jq -cn --args '$ARGS.positional' "${unpublished[@]}")"
echo "packages=$plan"
