#!/usr/bin/env bash
# Deploy MarkdownFS to a Hugging Face Space.
#
# Usage:  huggingface/deploy.sh <hf-username> <space-name>
# Needs:  HF_TOKEN env var with write scope (https://huggingface.co/settings/tokens)
#
# Idempotent: safe to re-run. The Space repo is checked out under
# .hf-space/ at the repo root and reused on subsequent runs.
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 <hf-username> <space-name>" >&2
  exit 2
fi

USER=$1
NAME=$2

if [[ -z "${HF_TOKEN:-}" ]]; then
  echo "HF_TOKEN env var is required (https://huggingface.co/settings/tokens)" >&2
  exit 2
fi

ROOT=$(git rev-parse --show-toplevel)
WORK=$ROOT/.hf-space
REMOTE="https://${USER}:${HF_TOKEN}@huggingface.co/spaces/${USER}/${NAME}"

if [[ ! -d "$WORK/.git" ]]; then
  echo "==> cloning Space repo into $WORK"
  git clone "$REMOTE" "$WORK"
else
  echo "==> updating Space repo at $WORK"
  git -C "$WORK" remote set-url origin "$REMOTE"
  git -C "$WORK" fetch origin
  git -C "$WORK" checkout main 2>/dev/null || git -C "$WORK" checkout master
  git -C "$WORK" reset --hard origin/HEAD || true
fi

echo "==> copying source"
cp "$ROOT/huggingface/Dockerfile"      "$WORK/Dockerfile"
cp "$ROOT/huggingface/.dockerignore"   "$WORK/.dockerignore"
cp "$ROOT/huggingface/README.md"       "$WORK/README.md"
cp "$ROOT/Cargo.toml"                  "$WORK/Cargo.toml"
cp "$ROOT/Cargo.lock"                  "$WORK/Cargo.lock"
rm -rf "$WORK/src" "$WORK/tests" "$WORK/examples"
cp -R "$ROOT/src"      "$WORK/src"
cp -R "$ROOT/tests"    "$WORK/tests"
cp -R "$ROOT/examples" "$WORK/examples"

cd "$WORK"
git add -A
if git diff --cached --quiet; then
  echo "==> nothing to deploy (Space already up to date)"
  exit 0
fi

REV=$(git -C "$ROOT" rev-parse --short HEAD)
git commit -m "Deploy from markdownfs@${REV}"
git push origin HEAD
echo "==> deployed. https://huggingface.co/spaces/${USER}/${NAME}"
echo "==> first build takes ~5-10 min. Watch logs in the Space UI."
