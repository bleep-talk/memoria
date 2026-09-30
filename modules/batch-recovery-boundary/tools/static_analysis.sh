#!/usr/bin/env bash
set -euo pipefail

run() {
    local toolchain log status
    if [[ "$(uname -s)" == Darwin ]]; then
        toolchain=stable-aarch64-apple-darwin
    else
        toolchain=stable
    fi
    log="$(mktemp)"
    trap 'rm -f "$log"' RETURN
    status=0
    cargo +"$toolchain" clippy -p memoria-memory --all-targets -- -D warnings -A clippy::len_zero >"$log" 2>&1 || status=$?
    cat "$log"
    if [[ -n "${RMS_HUNT_OUTPUT:-}" ]]; then
        python3 - "$RMS_HUNT_OUTPUT" "$status" <<'PY'
import json,sys
status=int(sys.argv[2])
result={'spec':'rms/hunt-lane-result/v0.1','status':'pass' if status==0 else 'invalid','metrics':{'analyzer_runs':1},'artifacts':[]}
if status:
    result['reason']='Clippy returned diagnostics; inspect the runner output.'
with open(sys.argv[1],'w') as output: json.dump(result,output)
PY
    fi
    return "$status"
}

run
