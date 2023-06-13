const { initSync, rewrite } = wasm_bindgen;

const wasm = atob(window.wasmB64Encoded);

const wasmBuffer = new Uint8Array(wasm.length);
for (let i = 0; i < wasm.length; i++) {
  wasmBuffer[i] = wasm.charCodeAt(i);
}

initSync(wasmBuffer);
rewrite();
