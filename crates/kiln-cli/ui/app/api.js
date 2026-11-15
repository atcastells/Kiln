// Client of the local Kiln server. `fetch` is injectable for tests.
export class ApiError extends Error {
  constructor(status, message) {
    super(message);
    this.name = "ApiError";
    this.status = status;
  }
}

const FALLBACK = {
  403: "Kiln rechazó la petición: solo acepta órdenes desde su propia página.",
  404: "No existe lo que se pidió.",
  409: "Todavía no hay proyecto: termina el primer paso.",
};

async function failure(res) {
  let detail = "";
  try { detail = (await res.json())?.error ?? ""; } catch { /* no JSON body */ }
  if (res.status >= 500) return new ApiError(res.status, `Kiln tuvo un error interno${detail ? `: ${detail}` : "."}`);
  if (res.status === 400 && detail) return new ApiError(400, detail);
  return new ApiError(res.status, FALLBACK[res.status] ?? detail ?? `La petición falló (${res.status}).`);
}

export function createApi({ fetch: f = (...a) => globalThis.fetch(...a), base = "" } = {}) {
  const cache = new Map();    // path -> { etag, value }
  const inflight = new Map(); // path -> Promise

  async function send(url, init) {
    try { return await f(base + url, init); }
    catch { throw new ApiError(0, "No se puede conectar con Kiln. ¿Sigue en marcha el servidor?"); }
  }

  async function load(path) {
    const hit = cache.get(path);
    const res = await send(path, hit ? { headers: { "If-None-Match": hit.etag } } : {});
    if (res.status === 304 && hit) return hit.value;
    if (!res.ok) throw await failure(res);
    const value = await res.json();
    const etag = res.headers.get("etag");
    if (etag) cache.set(path, { etag, value }); else cache.delete(path);
    return value;
  }

  return {
    getJSON(path) {
      let p = inflight.get(path);
      if (!p) {
        p = load(path).finally(() => inflight.delete(path));
        inflight.set(path, p);
      }
      return p;
    },
    // The server only accepts writes that carry this header (see serve.rs).
    async post(path, body) {
      const res = await send(path, {
        method: "POST",
        headers: { "Content-Type": "application/json", "X-Kiln": "1" },
        body: JSON.stringify(body ?? {}),
      });
      if (!res.ok) throw await failure(res);
      return res.json();
    },
  };
}

const shared = createApi();
export const getJSON = shared.getJSON;
export const post = shared.post;
