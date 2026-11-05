#!/usr/bin/env bash
# 58-registry-probe.sh -- read-only crates.io registry probe for the Phase 58 release (D-00f, D-00h).
#
# Usage:
#   58-registry-probe.sh --version <V> --mode baseline|absent|present [--trustpub]
#
# Modes:
#   baseline  one tab-separated row per crate: name, newest version line, its cksum, its yanked,
#             has_<V>=yes|no
#   absent    fails if any crate has a sparse-index line whose vers equals V
#   present   prints name, vers, cksum, yanked for the V line; fails unless each crate has exactly
#             one such line with yanked=false and a 64-hex cksum. --trustpub adds the crates.io API
#             trustpub_data fields (provider, repository, run_id, sha) per crate.
#
# The publishable set is derived at run time from `cargo metadata --no-deps` (publish == null);
# no crate name is written into this script. An empty derived set exits 2, and the number of
# printed rows must equal the number of derived crates, so an empty registry answer is a named
# failure and never a pass (T-58-07). Read-only: HTTPS GET only, the mandatory User-Agent is the
# only header, no credential of any kind (D-00g), no redirect following.
set -euo pipefail

UA='paladin-release-verify (github.com/DF3NDR/paladin-dev-env)'
INDEX_BASE='https://index.crates.io'
API_BASE='https://crates.io/api/v1/crates'

version=''
mode=''
trustpub=0

usage() {
  echo "usage: $0 --version <V> --mode baseline|absent|present [--trustpub]" >&2
  exit 64
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --version) [ "$#" -ge 2 ] || usage; version="$2"; shift 2 ;;
    --mode) [ "$#" -ge 2 ] || usage; mode="$2"; shift 2 ;;
    --trustpub) trustpub=1; shift ;;
    *) usage ;;
  esac
done

[ -n "$version" ] || usage
case "$mode" in
  baseline | absent | present) ;;
  *) usage ;;
esac
if [ "$trustpub" -eq 1 ] && [ "$mode" != present ]; then
  echo "ERROR: --trustpub is only valid with --mode present" >&2
  exit 64
fi

# Sparse-index path rule: 1, 2 or 3 character names use 1/<n>, 2/<n>, 3/<first char>/<n>;
# 4 or more use <n[0:2]>/<n[2:4]>/<n>.
sparse_path() {
  local n="$1" len="${#1}"
  if [ "$len" -eq 1 ]; then
    echo "1/$n"
  elif [ "$len" -eq 2 ]; then
    echo "2/$n"
  elif [ "$len" -eq 3 ]; then
    echo "3/${n:0:1}/$n"
  else
    echo "${n:0:2}/${n:2:2}/$n"
  fi
}

mapfile -t crates < <(
  cargo metadata --format-version 1 --no-deps |
    jq -r '.packages[] | select(.publish == null) | .name' |
    LC_ALL=C sort
)

total="${#crates[@]}"
if [ "$total" -eq 0 ] || [ -z "${crates[0]:-}" ]; then
  echo 'ERROR: no publishable crate derived' >&2
  exit 2
fi

rows=0
failures=0

fail() {
  echo "FAIL: $*" >&2
  failures=$((failures + 1))
}

for c in "${crates[@]}"; do
  url="$INDEX_BASE/$(sparse_path "$c")"
  if ! body=$(curl -sS -f -A "$UA" "$url"); then
    fail "$c: sparse-index read failed ($url)"
    continue
  fi
  if [ -z "$body" ]; then
    fail "$c: empty sparse-index answer ($url)"
    continue
  fi

  case "$mode" in
    baseline)
      newest=$(printf '%s\n' "$body" | tail -n 1)
      has=$(printf '%s\n' "$body" | jq -r --arg v "$version" 'select(.vers == $v) | .vers' | wc -l)
      has_label=no
      [ "$has" -gt 0 ] && has_label=yes
      printf '%s\t%s\n' "$c" "$(printf '%s' "$newest" |
        jq -r --arg h "has_${version}=${has_label}" '[.vers, .cksum, .yanked, $h] | @tsv')"
      rows=$((rows + 1))
      ;;
    absent)
      hits=$(printf '%s\n' "$body" | jq -r --arg v "$version" 'select(.vers == $v) | .vers' | wc -l)
      if [ "$hits" -gt 0 ]; then
        printf '%s\tPRESENT\n' "$c"
        fail "$c: a $version line exists on the registry"
      else
        printf '%s\tABSENT\n' "$c"
      fi
      rows=$((rows + 1))
      ;;
    present)
      line_count=$(printf '%s\n' "$body" | jq -r --arg v "$version" 'select(.vers == $v) | .vers' | wc -l)
      if [ "$line_count" -ne 1 ]; then
        fail "$c: expected exactly one $version line, found $line_count"
        printf '%s\tMISSING_OR_DUPLICATE(%s)\n' "$c" "$line_count"
        rows=$((rows + 1))
        continue
      fi
      row=$(printf '%s\n' "$body" | jq -r --arg v "$version" \
        'select(.vers == $v) | [.vers, .cksum, .yanked] | @tsv')
      cksum=$(printf '%s' "$row" | cut -f2)
      yanked=$(printf '%s' "$row" | cut -f3)
      if [ "$yanked" != false ]; then
        fail "$c: $version is yanked"
      fi
      if ! printf '%s' "$cksum" | grep -Eq '^[0-9a-f]{64}$'; then
        fail "$c: cksum is not 64 hex characters"
      fi
      extra=''
      if [ "$trustpub" -eq 1 ]; then
        if ! api=$(curl -sS -f -A "$UA" "$API_BASE/$c/$version"); then
          fail "$c: crates.io API read failed"
          api=''
        fi
        if [ -n "$api" ]; then
          extra=$(printf '%s' "$api" | jq -r \
            '.version.trustpub_data // {} | [(.provider // "null"), (.repository // "null"), (.run_id // "null"), (.sha // "null")] | @tsv')
          provider=$(printf '%s' "$extra" | cut -f1)
          if [ "$provider" = null ]; then
            fail "$c: no trustpub_data on the registry"
          fi
        fi
      fi
      if [ -n "$extra" ]; then
        printf '%s\t%s\t%s\n' "$c" "$row" "$extra"
      else
        printf '%s\t%s\n' "$c" "$row"
      fi
      rows=$((rows + 1))
      ;;
  esac
done

if [ "$rows" -ne "$total" ]; then
  fail "printed rows ($rows) differ from derived crates ($total)"
fi

if [ "$failures" -ne 0 ]; then
  echo "probe: $total crate(s), mode $mode, version $version: FAIL ($failures)"
  exit 1
fi
echo "probe: $total crate(s), mode $mode, version $version: OK"
