// A small keyed store. Subscribers name the keys they use and run only when
// one of those keys holds a different value (identity) than the last time they
// ran. Notifications are batched in one microtask.
export function createStore(initial = {}) {
  const state = { ...initial };
  const subs = new Set();
  let dirty = new Set();
  let queued = false;

  function flush() {
    queued = false;
    const touched = dirty;
    dirty = new Set();
    for (const s of [...subs]) {
      if (!s.active) continue;
      const changed = [];
      for (const k of s.keys) {
        if (touched.has(k) && !Object.is(s.seen.get(k), state[k])) changed.push(k);
      }
      if (!changed.length) continue;
      for (const k of changed) s.seen.set(k, state[k]);
      s.fn(changed);
    }
  }

  function set(key, value) {
    if (Object.is(state[key], value)) return;
    state[key] = value;
    dirty.add(key);
    if (!queued) { queued = true; queueMicrotask(flush); }
  }

  return {
    get: key => state[key],
    set,
    update: (key, f) => set(key, f(state[key])),
    subscribe(keys, fn) {
      const list = Array.isArray(keys) ? keys : [keys];
      const s = { keys: new Set(list), fn, active: true, seen: new Map(list.map(k => [k, state[k]])) };
      subs.add(s);
      return () => { s.active = false; subs.delete(s); };
    },
  };
}

export const store = createStore({ catalog: null, context: null, route: null, query: "", job: null, selection: null });
