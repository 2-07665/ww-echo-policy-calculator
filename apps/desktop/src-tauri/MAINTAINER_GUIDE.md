# Desktop adapter

The desktop hosts `apps/web/` directly. It does not need a running HTTP server or Node build.

- `src/main.rs`: three commands (`bootstrap`, `preview`, `solve`) delegate to `echo_app`.
- `src/ocr.rs`: optional loopback UDP 9999 listener for the existing ok-ww packet format.
- `echo_app::Service`: shared solver, validation, DTOs and a one-configuration cache.
- `apps/web/api.js`: selects Tauri IPC or same-origin HTTP; all other UI code is shared.

Heavy solves use a blocking worker, not the UI thread. The solver mutex uses `try_lock` so concurrent calls fail explicitly. Browser/server calculations use the same service, with a single semaphore permit held until the blocking calculation actually ends.

The product identifier is unchanged so the desktop keeps its existing webview origin. Legacy user preset files are not deleted. Import them using the shared UI, after selecting their scoring system; see the root README.

Check with `cargo check -p echo_calculator_app`, `cargo test -p echo_calculator_app` and `bash scripts/check-ui-node.sh`. Test shared numerical behavior with `cargo test -p echo_app`. Keep platform-specific capabilities in this adapter; do not reintroduce separate scoring or solver implementations here.
