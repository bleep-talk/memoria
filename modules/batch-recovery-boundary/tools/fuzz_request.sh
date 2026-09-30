#!/usr/bin/env bash
set -euo pipefail

prepare_corpus() {
    cp fuzz/corpus/request_validation/* "$1/"
}

run() {
    local seed="${RMS_HUNT_SEED:-1}"
    local seconds="${RMS_HUNT_BUDGET_SECONDS:-5}"
    [[ "$seed" =~ ^[0-9]+$ ]] || { echo 'invalid hunt seed' >&2; return 2; }
    [[ "$seconds" =~ ^[1-9][0-9]*$ ]] || { echo 'invalid hunt budget' >&2; return 2; }
    local scratch corpus artifacts log toolchain status
    scratch="$(mktemp -d)"
    trap 'rm -rf "$scratch"' RETURN
    corpus="$scratch/corpus"
    log="$scratch/libfuzzer.log"
    mkdir -p "$corpus"
    prepare_corpus "$corpus"
    mkdir -p fuzz/artifacts/request_validation
    artifacts="$(mktemp -d fuzz/artifacts/request_validation/run.XXXXXX)"
    if [[ "$(uname -s)" == Darwin ]]; then
        toolchain=nightly-aarch64-apple-darwin
    else
        toolchain=nightly
    fi
    status=0
    cargo +"$toolchain" fuzz run request_validation "$corpus" -- -max_total_time="$seconds" -max_len=4096 -seed="$seed" -artifact_prefix="$artifacts/" >"$log" 2>&1 || status=$?
    cat "$log"
    if [[ -n "${RMS_HUNT_OUTPUT:-}" ]]; then
        python3 - "$log" "$RMS_HUNT_OUTPUT" "$status" "$artifacts" <<'PY'
import glob,json,re,sys
log=open(sys.argv[1],errors='replace').read()
path=sys.argv[2]
status=int(sys.argv[3])
artifacts=sorted(glob.glob(sys.argv[4]+'/*'))
cases=re.findall(r'Done (\d+) runs',log)
coverage=re.findall(r'(?:DONE|INITED)\s+cov: (\d+)',log)
result={'spec':'rms/hunt-lane-result/v0.1','status':'pass' if status==0 else ('finding' if artifacts else 'invalid'),'metrics':{'cases':int(cases[-1]) if cases else 0,'coverage_edges':int(coverage[-1]) if coverage else 0},'artifacts':artifacts}
if status:
    if artifacts:
        result['findings']=[{'kind':'crash','summary':'LibFuzzer found a failing input in fuzz/artifacts/request_validation.'}]
    else:
        result['reason']='LibFuzzer did not complete and produced no replayable artifact.'
with open(path,'w') as output: json.dump(result,output)
PY
    fi
    if [[ "$status" -eq 0 ]]; then
        rmdir "$artifacts"
    fi
    return "$status"
}

run
