# CLI stdio authority evidence

`cli/src/main.rs#run` parses requests, reads write content from stdin, calls `MemoryRepo`, and writes JSON results to stdout or diagnostics to stderr. The CLI exposes no policy override and calls no filesystem or Git executor directly.

Observed proof: the macOS and Linux workspace suites include CLI JSON/error tests. `json_success_and_request_errors_are_separate_streams` checks the versioned result envelope, error codes, and stderr separation. `invalid_utf8_stdin_is_a_typed_request_error` checks malformed input. A manual init → write → context → diff → commit → log journey passed. The release binary was built and an independent consumer imported the public library from the source archive.
