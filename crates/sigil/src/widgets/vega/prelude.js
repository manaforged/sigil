globalThis.__timers = [];
globalThis.setTimeout = (f, d, ...a) => { __timers.push(() => f(...a)); return __timers.length; };
globalThis.clearTimeout = () => {};
globalThis.setInterval = () => 0;
globalThis.clearInterval = () => {};
globalThis.__tick = () => { const t = __timers.shift(); if (!t) return false; t(); return true; };
globalThis.console = { log() {}, warn() {}, error() {}, info() {}, debug() {} };
globalThis.self = globalThis;
globalThis.structuredClone = function clone(v, seen = new Map()) {
  if (v === null || typeof v !== 'object') return v;
  if (seen.has(v)) return seen.get(v);
  let o;
  if (Array.isArray(v)) { o = []; seen.set(v, o); for (let i = 0; i < v.length; i++) o[i] = clone(v[i], seen); return o; }
  if (v instanceof Date) return new Date(v.getTime());
  if (v instanceof RegExp) return new RegExp(v.source, v.flags);
  if (v instanceof Map) { o = new Map(); seen.set(v, o); v.forEach((x, k) => o.set(clone(k, seen), clone(x, seen))); return o; }
  if (v instanceof Set) { o = new Set(); seen.set(v, o); v.forEach((x) => o.add(clone(x, seen))); return o; }
  o = {}; seen.set(v, o);
  for (const k of Object.keys(v)) o[k] = clone(v[k], seen);
  return o;
};
(() => {
  const locale = Number.prototype.toLocaleString;
  Number.prototype.toLocaleString = function (...a) {
    const v = Number(this);
    if (v === Infinity) return '∞';
    if (v === -Infinity) return '-∞';
    return locale.apply(this, a);
  };
})();
