@echo off
echo Building iris-sfc-wasm...
cd /d "%~dp0"
wasm-pack build --target nodejs --release
echo Done! WASM module built to pkg/
