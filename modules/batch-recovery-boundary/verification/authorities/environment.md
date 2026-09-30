# Environment authority

`src/journal.rs#crash` reads the `MEMORIA_TEST_CRASH` process environment variable at selected native journal points. `src/effects/apply_filesystem.rs#execute` reaches this helper during file application. The canonical effect rows include `environment` wherever that helper is transitively reachable. This authority does not select recovery decisions; it supplies a test fault fact by terminating the process at a named point.

Observed proof: the macOS and Linux workspace suites passed with the fault variable absent. Process-death integration tests set named crash points, restart, and check rollback or cleanup. The fault hook is local and does not read network state.
