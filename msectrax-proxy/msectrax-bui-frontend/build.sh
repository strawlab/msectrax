#!/bin/bash -x
set -o errexit

cargo web build --release

mkdir -p dist
cp -a target/wasm32-unknown-unknown/release/msectrax-bui-frontend.js target/wasm32-unknown-unknown/release/msectrax-bui-frontend.wasm dist/
cd dist
cp ../static/index.html .
cp ../static/style.css .

