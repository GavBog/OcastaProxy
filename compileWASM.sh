cargo build --target wasm32-unknown-unknown --release -p wasm
wasm-bindgen --target no-modules --no-typescript --out-dir ./target/wasm32-unknown-unknown/release ./target/wasm32-unknown-unknown/release/wasm.wasm
cp ./target/wasm32-unknown-unknown/release/wasm_bg.wasm static/wasm_bg.wasm
cp ./target/wasm32-unknown-unknown/release/wasm.js static/wasm.js
