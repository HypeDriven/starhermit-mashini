#!/usr/bin/env bash
# Runs mashini's tests as a k3s Job (never on this machine).
#   tools/k3s-test.sh [--host hd-N] [CMD]     (CMD defaults to npm test's `cargo test --release --offline`)
# Cargo.toml path-depends on ../../sh/starhermit-engine/crates/*, outside the game, so the job's tree is a
# staged copy of that layout: games/mashini (this repo, no target/) plus sh/starhermit-engine as `git archive`
# of the engine tag matching engine.version (read-only on the engine repo; the archive has the whole
# workspace, so inherited `.workspace = true` fields and members resolve). Registry crates are fetched first.
set -euo pipefail
here=$(cd "$(dirname "$0")/.." && pwd)
HOST=hd-2; [ "${1:-}" = --host ] && { HOST=$2; shift 2; }
CMD=${1:-cargo test --release --offline}
ver=$(grep -m1 '^engine.version=' "$here/starhermit.txt" | cut -d= -f2)
stage=${TMPDIR:-/tmp}/mashini-ci-$ver
rm -rf "$stage/games"; mkdir -p "$stage/games" "$stage/sh/starhermit-engine"
if [ ! -f "$stage/sh/starhermit-engine/Cargo.toml" ]; then
  git -C ~/sh/starhermit-engine archive "v$ver" | tar -x -C "$stage/sh/starhermit-engine"
fi
rsync -a --exclude target --exclude .git "$here" "$stage/games/"
# K3S_FETCH="games/mashini/mashini.wasm …" copies those paths back into K3S_OUT (default ./sh-job-out/mashini-ci).
fetch=(); for f in ${K3S_FETCH:-}; do fetch+=(--fetch "$f"); done; [ -n "${K3S_OUT:-}" ] && fetch+=(--out "$K3S_OUT")
exec ~/sh/runtime/ci/sh-job.py --host "$HOST" --image "runtime:$ver" --ttl 172800 --cpu 8 --mem 12Gi "${fetch[@]}" \
  --name mashini-ci --tree "$stage" --timeout 3600 -- \
  "set -eo pipefail; cd games/mashini; for i in 1 2 3; do cargo fetch -q && break; sleep 10; done; $CMD"
