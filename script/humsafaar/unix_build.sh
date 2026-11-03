#!/usr/bin/env bash
# This works on Linux and MacOS to build production Humsafaar

set -euxo pipefail

echo "Building production Humsafaar..."
echo ""

root_dir=$(pwd)
frontend_path="${root_dir}/frontend"
rust_crate_path="${root_dir}/crates/desktop/humsafaar"

# Build with the desktop app configuration.
config_path="${rust_crate_path}/tauri.conf.json"

pushd "${frontend_path}" || exit

npm install --verbose

popd || exit

export TAURI_FRONTEND_PATH="${frontend_path}"
export TAURI_APP_PATH="${rust_crate_path}"

# NB: The "frontend dev" script sets "production" too, so this must only control the
# hostnames we use, not minification, etc.
export VITE_ENVIRONMENT_TYPE="production"
export SQLX_OFFLINE=true

# This appears to trigger "nx build" instead of "nx dev".
cargo tauri build --config "${config_path}"

echo "Done!"

date "+Finished on %A, %B %e - %H:%M:%S (local timezone)"

