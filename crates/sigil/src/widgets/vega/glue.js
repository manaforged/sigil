vega.textMetrics.width = (item, text) => {
  const weight = item.fontWeight == null ? '' : String(item.fontWeight);
  const style = item.fontStyle == null ? '' : String(item.fontStyle);
  const w = __measure(String(text), vega.fontSize(item), String(vega.fontFamily(item) || ''), weight, style);
  return item.limit > 0 ? Math.min(w, item.limit) : w;
};

const __quiet = { level(l) { return l == null ? 0 : this; }, error() { return this; }, warn() { return this; }, info() { return this; }, debug() { return this; } };

const __refuse = (errors) => {
  const loader = vega.loader({ mode: 'http' });
  const refuse = async (uri) => { errors.push('data URL refused: ' + uri); throw new Error('data URLs are not loaded'); };
  loader.load = refuse;
  loader.http = refuse;
  loader.file = refuse;
  loader.sanitize = refuse;
  return loader;
};

const __titled = new Set(['axis-title', 'column-title', 'row-title']);

function __titles(mark, out, titled = false) {
  if (!mark || !mark.items) return out;
  const on = titled || __titled.has(mark.role);
  for (const item of mark.items) {
    if (on && mark.marktype === 'text' && item.text != null && item.opacity !== 0) {
      const text = Array.isArray(item.text) ? item.text.join(' ') : String(item.text);
      if (text && !out.includes(text)) out.push(text);
    }
    if (item.items) for (const child of item.items) __titles(child, out, on);
  }
  return out;
}

globalThis.__render = function (specText, configText) {
  globalThis.__done = false;
  globalThis.__out = null;
  __timers.length = 0;
  vega.setRandom(vega.randomLCG(20261105));
  const out = {};
  const errors = [];
  const finish = () => { if (errors.length) out.err = errors.join('; '); __out = out; __done = true; };
  const logger = Object.assign({}, __quiet, { error(...m) { errors.push(String((m[0] && m[0].message) || m[0]).slice(0, 200)); return this; } });
  let view;
  try {
    const config = JSON.parse(configText);
    const spec = vegaLite.compile(JSON.parse(specText), { config, logger: __quiet }).spec;
    view = new vega.View(vega.parse(spec), { renderer: 'none', loader: __refuse(errors), logger, logLevel: vega.Error });
  } catch (e) { out.err = String(e); __out = out; __done = true; return; }
  view.runAsync()
    .then(() => { try { view.signal('geo_interval_init_tick', {}); } catch (e) {} })
    .then(() => view.runAsync())
    .then(() => view.toSVG())
    .then((svg) => { out.svg = svg; out.text = __titles(view.scenegraph().root, []).join('\n'); })
    .catch((e) => { errors.push(String(e)); })
    .finally(() => { try { view.finalize(); } catch (e) {} finish(); });
};
