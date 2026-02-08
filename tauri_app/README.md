# Tauri App Layer

This folder contains the new desktop app layer for the Echo Calculator migration.

## Structure

- `src-tauri/`: Rust desktop host and command handlers.
- `ui/`: Static frontend assets served by Tauri.

## Notes

- The solver core remains in `../rust` and is consumed as a path dependency.
- No Python runtime is required in this main UI path.
- OCR is intentionally out of scope for this layer and can be integrated later via IPC/HTTP.
