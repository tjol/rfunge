#!/bin/sh

cd "`dirname $0`"

wasm-pack build --target=web --out-dir=www/wasm_pkg && \
git archive --format=tar --prefix=rfunge/ HEAD | xz - > www/rfunge.src.tar.xz && \
git rev-parse --short=8 HEAD > www/wasm_pkg/rev.hash.txt
