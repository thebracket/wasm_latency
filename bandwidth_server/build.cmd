REM Build system for Windows
@echo off

echo Building WASM
cd ../wasm_client
cargo build --release --target wasm32-unknown-unknown

echo Running WASM bindgen
mkdir staging
wasm-bindgen --target web --out-dir staging ..\target\wasm32-unknown-unknown\release\wasm_client.wasm

echo Copying WASM to staging area
copy staging\* ..\bandwidth_site\wasm

echo "Building the TypeScript site"
cd ..\bandwidth_site
node .\esbuild.mjs

cd ../bandwidth_server
cargo build --release
cargo run --release
