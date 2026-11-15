// Entry of the new UI. For now: proves that modules load from the binary and
// that the API client reaches the server.
import { getJSON } from "./api.js";
import { store } from "./store.js";

const name = document.getElementById("kiln-project");
store.subscribe("catalog", () => { name.textContent = store.get("catalog").project; });

try {
  store.set("catalog", await getJSON("/api/catalog"));
} catch (e) {
  const box = document.getElementById("kiln-error");
  box.textContent = e.message;
  box.hidden = false;
}
