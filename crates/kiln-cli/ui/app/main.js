// Entry of the new UI (/app): wires the store, the API client, the router and
// the view registry into the shell and starts it.
import * as api from "./api.js";
import { store } from "./store.js";
import { createRouter, VIEWS } from "./router.js";
import { REGISTRY } from "./views/registry.js";
import { createShell } from "./shell.js";

createShell({ store, api, router: createRouter({ views: VIEWS }), registry: REGISTRY }).start();
