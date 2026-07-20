const loading = document.querySelector("#loading");

async function start() {
  try {
    const wasm = await import("./wasm/teshi_web.js");
    await wasm.default();
    wasm.run();
    loading.hidden = true;
  } catch (error) {
    console.error("Failed to initialize teshi:", error);
    loading.classList.add("error");
    loading.textContent = "teshi could not start. Please refresh the page or use a current browser.";
  }
}

start();
