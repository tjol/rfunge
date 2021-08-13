#!/bin/sh

cd "`dirname $0`"

exec wasm-pack build --target=web --out-dir=www/wasm_pkg
