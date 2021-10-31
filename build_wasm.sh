#!/bin/sh

cd "`dirname $0`"

hash=$(git rev-parse --short=8 HEAD)
wasm-pack build --target=web --out-dir=webapp/rfunge_wasm && \
echo "VITE_RFUNGE_GIT_HASH=${hash}" > webapp/.env.local
