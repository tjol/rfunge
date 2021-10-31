#!/bin/sh

cd "`dirname $0`"

wasm-pack build --target=web --out-dir=webapp/rfunge_wasm && (
hash=$(git rev-parse --short=8 HEAD)
echo "VITE_RFUNGE_GIT_HASH=${hash}" > webapp/.env.local
)
