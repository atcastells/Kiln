import test from "node:test";
import assert from "node:assert/strict";
import { createApi, ApiError } from "../../app/api.js";

const res = (status, body, headers = {}) => ({
  status, ok: status >= 200 && status < 300,
  headers: { get: k => headers[k.toLowerCase()] ?? null },
  json: async () => { if (body === undefined) throw new SyntaxError("no body"); return JSON.parse(JSON.stringify(body)); },
});
const recorder = (...replies) => {
  const calls = [];
  const fetch = async (url, init = {}) => { calls.push({ url, init }); const r = replies.shift(); if (r instanceof Error) throw r; return typeof r === "function" ? r() : r; };
  return { fetch, calls };
};

test("getJSON sends If-None-Match after the first answer and reuses the value on 304", async () => {
  const { fetch, calls } = recorder(res(200, { n: 1 }, { etag: '"abc"' }), res(304));
  const api = createApi({ fetch });
  const a = await api.getJSON("/api/catalog");
  assert.deepEqual(a, { n: 1 });
  assert.equal(calls[0].init.headers?.["If-None-Match"], undefined);
  const b = await api.getJSON("/api/catalog");
  assert.equal(b, a, "same cached object on 304");
  assert.equal(calls[1].init.headers["If-None-Match"], '"abc"');
});

test("a 200 after a cached etag replaces the cached value", async () => {
  const { fetch } = recorder(res(200, { n: 1 }, { etag: '"a"' }), res(200, { n: 2 }, { etag: '"b"' }), res(304));
  const api = createApi({ fetch });
  await api.getJSON("/x");
  assert.deepEqual(await api.getJSON("/x"), { n: 2 });
  assert.deepEqual(await api.getJSON("/x"), { n: 2 });
});

test("identical GETs in flight share one request", async () => {
  let release;
  const gate = new Promise(r => { release = r; });
  const { fetch, calls } = recorder(() => gate.then(() => res(200, { ok: 1 })), res(200, { ok: 2 }), res(200, { ok: 3 }));
  const api = createApi({ fetch });
  const p1 = api.getJSON("/api/context"), p2 = api.getJSON("/api/context"), p3 = api.getJSON("/api/other");
  release();
  const [a, b] = await Promise.all([p1, p2]);
  assert.equal(a, b);
  assert.equal(calls.filter(c => c.url === "/api/context").length, 1);
  await p3.catch(() => {});
  await api.getJSON("/api/context");
  assert.equal(calls.filter(c => c.url === "/api/context").length, 2, "a later GET is a new request");
});

test("post adds the X-Kiln header and a JSON body", async () => {
  const { fetch, calls } = recorder(res(200, { ok: true }));
  const out = await createApi({ fetch }).post("/api/bind", { piece: "p" });
  assert.deepEqual(out, { ok: true });
  assert.equal(calls[0].init.method, "POST");
  assert.equal(calls[0].init.headers["X-Kiln"], "1");
  assert.equal(calls[0].init.headers["Content-Type"], "application/json");
  assert.equal(calls[0].init.body, '{"piece":"p"}');
});

test("errors are ApiError with the status and a Spanish message", async () => {
  const api = createApi({ fetch: recorder(res(400, { error: "falta el asset" }), res(403, { error: "solo desde la pagina de Kiln" }), res(404, undefined), res(500, undefined), res(409, { error: "x" })).fetch });
  await assert.rejects(api.post("/api/build", {}), e => e instanceof ApiError && e.status === 400 && e.message === "falta el asset");
  await assert.rejects(api.post("/api/build", {}), e => e instanceof ApiError && e.status === 403 && /Kiln/.test(e.message));
  await assert.rejects(api.getJSON("/nope"), e => e.status === 404 && /No existe/.test(e.message));
  await assert.rejects(api.getJSON("/boom"), e => e.status === 500 && /error/i.test(e.message));
  await assert.rejects(api.getJSON("/new"), e => e.status === 409);
});

test("a network failure is an ApiError with status 0", async () => {
  const api = createApi({ fetch: recorder(new TypeError("Failed to fetch")).fetch });
  await assert.rejects(api.getJSON("/api/catalog"), e => e instanceof ApiError && e.status === 0 && /conectar/.test(e.message));
});

test("a failed GET is not cached and the in-flight slot is freed", async () => {
  const { fetch, calls } = recorder(res(500, undefined), res(200, { ok: 1 }));
  const api = createApi({ fetch });
  await assert.rejects(api.getJSON("/a"));
  assert.deepEqual(await api.getJSON("/a"), { ok: 1 });
  assert.equal(calls[1].init.headers?.["If-None-Match"], undefined);
});

test("a fresh read sends the explicit fresh marker and sends no If-None-Match", async () => {
  const { fetch, calls } = recorder(res(200, { n: 1 }, { etag: '"a"' }), res(200, { n: 1 }, { etag: '"a"' }));
  const api = createApi({ fetch });
  await api.getJSON("/api/catalog");
  const again = await api.getJSON("/api/catalog", { fresh: true });
  assert.deepEqual(again, { n: 1 });
  assert.equal(calls[1].init.headers["X-Kiln-Fresh"], "1");
  assert.equal(calls[1].init.headers["If-None-Match"], undefined);
});
