#!/usr/bin/env sh
# Builds the browser simulator into sim/web/generated/.
set -eu
cd "$(dirname "$0")/.."

cargo build --release --locked -p pinball-sim --target wasm32-unknown-unknown
mkdir -p sim/web/generated
cp target/wasm32-unknown-unknown/release/pinball_sim.wasm sim/web/generated/
cp docs/playfield.svg sim/web/generated/

echo "Built. Run: python3 -m http.server -d sim/web 8000"
echo "Then open http://localhost:8000"
