#!/usr/bin/env bash
set -euo pipefail

run() {
    local selector="${1:-${RMS_PROPERTY_RUNNER:-}}"
    case "$selector" in
        tests/properties.rs#*)
            just memoria-property "$selector"
            ;;
        fuzz/fuzz_targets/request_validation.rs#fuzz_one)
            bash tools/fuzz_request.sh
            ;;
        tools/static_analysis.sh#run)
            bash tools/static_analysis.sh
            ;;
        *)
            printf 'unknown RMS property runner: %s\n' "$selector" >&2
            return 2
            ;;
    esac
}

run "$@"
