const { rewrite } = wasm_bindgen;
const page = atob(ocastaPage);
wasm_bindgen().then(() => {
  rewrite();

  document.open();
  document.write(page);
  document.close();

  const scripts = document.querySelectorAll("[data-ocasta]");
  scripts.forEach((script) => {
    script.remove();
  });
});
