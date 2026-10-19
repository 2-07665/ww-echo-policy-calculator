#!/usr/bin/env bash
set -euo pipefail
NODE_BIN="${NODE_BIN:-node}"
for file in apps/web/*.js; do
  "$NODE_BIN" --input-type=module --check < "$file"
done
echo "Shared UI syntax checks passed."
