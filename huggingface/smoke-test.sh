#!/usr/bin/env bash
# Smoke-test a deployed MarkdownFS Hugging Face Space.
#
# Usage:  huggingface/smoke-test.sh <space-url>
#         huggingface/smoke-test.sh https://your-username-markdownfs.hf.space
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <space-url>" >&2
  exit 2
fi

SPACE=${1%/}

step() { printf "\n==> %s\n" "$*"; }
ok()   { printf "    ok\n"; }

step "GET /health"
curl -fsS "$SPACE/health" | tee /dev/stderr | grep -q '"status":"ok"'
ok

step "PUT /fs/notes/smoke.md"
curl -fsS -X PUT "$SPACE/fs/notes/smoke.md" \
  -H 'content-type: text/markdown' \
  --data-binary "# smoke test $(date -u +%FT%TZ)" >/dev/null
ok

step "GET /fs/notes/smoke.md"
curl -fsS "$SPACE/fs/notes/smoke.md"
echo
ok

step "GET /fs/notes (list)"
curl -fsS "$SPACE/fs/notes" | grep -q '"name":"smoke.md"'
ok

step "GET /search/grep?pattern=smoke"
curl -fsS "$SPACE/search/grep?pattern=smoke&recursive=true" | grep -q '"count":'
ok

step "POST /vcs/commit"
HASH=$(curl -fsS -X POST "$SPACE/vcs/commit" \
  -H 'content-type: application/json' \
  -d '{"message":"smoke commit"}' | grep -o '"hash":"[^"]*' | cut -d'"' -f4)
echo "    hash=$HASH"
ok

step "GET /vcs/log"
curl -fsS "$SPACE/vcs/log" | grep -q "$HASH"
ok

step "DELETE /fs/notes/smoke.md"
curl -fsS -X DELETE "$SPACE/fs/notes/smoke.md" >/dev/null
ok

printf "\nall checks passed.\n"
