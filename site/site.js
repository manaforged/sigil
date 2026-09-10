import { apps, categories, components, coverageLabels, patterns, screenshotBase, showcases } from "./catalog.mjs";

const previewBase = "assets/components/";

const $ = (selector) => document.querySelector(selector);
const routeView = $("[data-route-view]");
const searchDialog = $("[data-command-dialog]");
const searchInput = $("[data-command-input]");
const searchResults = $("[data-command-results]");
const catalogDialog = $("[data-catalog-dialog]");
const copyNotice = $("[data-copy-notice]");
let activeResult = 0;

const escapeHtml = (value) => String(value).replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;");
const icon = (name) => `<svg class="inline-icon" viewBox="0 0 24 24" aria-hidden="true">${name === "arrow" ? '<path d="m9 18 6-6-6-6"></path>' : name === "copy" ? '<rect x="8" y="8" width="11" height="11" rx="2"></rect><path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2"></path>' : '<path d="M14 5h5v5M10 14 19 5"></path><path d="M19 13v5a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1h5"></path>'}</svg>`;

function commandBlock(command, label = "Copy") {
  return `<div class="code-block"><code>${escapeHtml(command)}</code><button type="button" data-copy="${escapeHtml(command)}" aria-label="Copy command">${icon("copy")}<span>${label}</span></button></div>`;
}

function catalogueMarkup() {
  return categories.map((category) => {
    const links = components.filter((item) => item.category === category.name).map((item) => `<a href="#/components/${item.slug}" data-component-link="${item.slug}">${item.name}</a>`).join("");
    return `<section class="catalog-group"><h2>${category.name}<span>${category.count}</span></h2>${links}</section>`;
  }).join("");
}

function componentDemo(component) {
  if (component.slug === "button") return buttonLab();
  if (!component.preview) return '<p class="preview-pending">A presentation capture has not been authored for this component yet. The usage and source below are current.</p>';
  const preview = `<img src="${previewBase}${component.preview}" alt="${component.name} rendered by Sigil on the standard documentation canvas">`;
  return `<section class="component-demo" aria-label="${component.name} example"><div class="demo-toolbar"><div class="demo-tabs" role="tablist" aria-label="Example view"><button type="button" role="tab" aria-selected="true" data-demo-tab="preview">Preview</button><button type="button" role="tab" aria-selected="false" data-demo-tab="code">Code</button></div><span data-stage-label>2× capture</span></div><div class="demo-panel" role="tabpanel" data-demo-panel="preview">${preview}</div><div class="demo-panel demo-panel--code" role="tabpanel" data-demo-panel="code" hidden><pre><code>${escapeHtml(component.code)}</code></pre></div></section>`;
}

const buttonDefaults = { label: "Save changes", variant: "primary", width: "148", disabled: false };

function buttonSnippet(state) {
  const variant = state.variant === "primary" ? ".primary()" : state.variant === "neutral" ? ".neutral()" : state.variant === "ghost" ? ".ghost()" : ".destructive()";
  const enabled = state.disabled ? "\n    .enabled(false)" : "";
  return `button(${JSON.stringify(state.label || "Button")})\n    ${variant}\n    .width(${state.width}.0)${enabled}\n    .activate(save_changes)`;
}

function buttonLab() {
  const code = buttonSnippet(buttonDefaults);
  const variants = ["primary", "neutral", "ghost", "destructive"].map((variant) => `<label><input class="visually-hidden" type="radio" name="variant" value="${variant}"${variant === "primary" ? " checked" : ""}><span>${variant[0].toUpperCase()}${variant.slice(1)}</span></label>`).join("");
  return `<section class="component-demo button-lab" aria-label="Interactive Button example" data-button-lab>
    <div class="demo-toolbar"><div class="demo-tabs" role="tablist" aria-label="Example view"><button type="button" role="tab" aria-selected="true" data-demo-tab="preview">Preview</button><button type="button" role="tab" aria-selected="false" data-demo-tab="code">Code</button></div><span><i></i> Live · WASM · WebGPU · DPR-aware</span></div>
    <div class="demo-panel button-lab__body" role="tabpanel" data-demo-panel="preview">
      <div class="button-lab__stage"><iframe src="live/index.html" title="Interactive Sigil Button rendered with WASM and WebGPU"></iframe></div>
      <form class="button-lab__controls">
        <div class="lab-heading"><span>Configure</span><button type="reset">Reset</button></div>
        <label class="lab-field"><span>Label</span><input name="label" type="text" value="Save changes" maxlength="24" autocomplete="off"></label>
        <fieldset class="lab-field"><legend>Variant</legend><div class="lab-segments">${variants}</div></fieldset>
        <label class="lab-field lab-range"><span>Width <output name="widthOutput">148 px</output></span><input name="width" type="range" min="96" max="280" step="4" value="148"></label>
        <label class="lab-check"><input name="disabled" type="checkbox"><span>Disabled</span></label>
        <p>Drag the width, change the label, then click the component. Every state is painted by the running Sigil binary.</p>
      </form>
    </div>
    <div class="demo-panel demo-panel--code button-lab__code" role="tabpanel" data-demo-panel="code" hidden><div><span>Generated from the current controls</span><button type="button" data-button-code-copy data-copy="${escapeHtml(code)}">${icon("copy")} Copy</button></div><pre><code data-button-code>${escapeHtml(code)}</code></pre></div>
  </section>`;
}

function componentCard(component) {
  return `<a class="component-card" href="#/components/${component.slug}"><div class="component-card__copy"><div><h3>${component.name}</h3>${component.preview ? '<span class="preview-mark">Preview</span>' : ""}</div><p>${component.description}</p><span>${coverageLabels[component.coverage]} ${icon("arrow")}</span></div></a>`;
}

function blockPreview(showcase, compact = false) {
  const source = apps.find((app) => app.slug === showcase.slug).source;
  return `<article class="block-preview${compact ? " block-preview--compact" : ""}"><div class="block-toolbar"><div><span class="toolbar-tab">Preview</span><p>${showcase.description}</p></div><div><code>${showcase.dimensions}</code><a href="${source}">Source ${icon("source")}</a></div></div><figure><img src="${screenshotBase}${showcase.screenshot}" alt="${showcase.alt}" loading="${compact ? "lazy" : "eager"}"></figure><div class="block-command"><code>${showcase.command}</code><span>Native macOS capture</span></div></article>`;
}

const pageHeader = (kicker, title, description) => `<header class="page-head"><p class="eyebrow">${kicker}</p><h1>${title}</h1><p>${description}</p></header>`;

function renderDocs() {
  const featured = ["button", "text-field", "table", "command-menu", "modal", "virtual-list"].map((slug) => componentCard(components.find((item) => item.slug === slug))).join("");
  const quickstart = '// Cargo.toml\nsigil-ui = { git = "https://github.com/ThomGardiner/sigil.git", tag = "v0.1.0" }\n\nuse sigil::prelude::*;\n\nlet count = signal(0);\n\ncolumn((\n    label("Counter"),\n    button("Increment")\n        .activate(move || count.update(\n            |value| *value += 1,\n        )),\n))';
  return `<article class="page page--landing">
    <section class="landing-hero"><div class="landing-hero__copy"><p class="eyebrow">Native desktop UI for Rust</p><h1>Own the interface.<br>Skip the webview.</h1><p>Sigil is a retained UI framework with owned layout, text, reactivity, accessibility, and wgpu rendering. Start with a component, then inspect the native evidence and source.</p><div class="action-row"><a class="action action--primary" href="#/components">Browse components ${icon("arrow")}</a><a class="action" href="https://github.com/ThomGardiner/sigil">View source ${icon("source")}</a></div></div><figure class="landing-hero__code"><pre><code>${escapeHtml(quickstart)}</code></pre><figcaption>Quickstart · Rust</figcaption></figure></section>
    <section class="inventory-band" aria-label="Current repository inventory"><div><strong>65</strong><span>public constructors</span></div><div><strong>28</strong><span>runnable examples</span></div><div><strong>116</strong><span>regression captures</span></div><div><strong>11</strong><span>documentation previews</span></div></section>
    <section class="native-section"><div class="section-heading"><div><p class="eyebrow">Application block</p><h2>Built with the same components.</h2></div><a href="#/apps">View all apps ${icon("arrow")}</a></div>${blockPreview(showcases[0])}</section>
    <section class="feature-section"><div class="section-heading"><div><p class="eyebrow">Component inventory</p><h2>Start from shipped behavior.</h2></div><a href="#/components">View all 65 ${icon("arrow")}</a></div><div class="featured-components">${featured}</div></section>
    <section class="principles"><div><p class="eyebrow">Rendering model</p><h2>Work follows change.</h2></div><div class="principles__grid"><article><span>01</span><h3>Retained tree</h3><p>The interface persists across frames. Signals mark the affected path instead of rebuilding the application tree.</p></article><article><span>02</span><h3>Owned pipeline</h3><p>Layout, shaping, line breaking, hit testing, and scene emission stay in one native pipeline.</p></article><article><span>03</span><h3>Idle means idle</h3><p>Layout and paint are gated on the reactive dirty set. An unchanged frame does no interface work.</p></article></div></section>
    <section class="start-section"><div><p class="eyebrow">Run the repository</p><h2>Open the current playground.</h2><p>It covers 32 of 65 public constructors. Use this site for the complete inventory and coverage gaps.</p></div>${commandBlock("cargo run -p widget-playground")}</section>
  </article>`;
}

function renderComponents() {
  const sections = categories.map((category) => `<section class="catalog-section"><div class="catalog-section__head"><h2>${category.name}</h2><span>${category.count}</span></div><div class="component-grid">${components.filter((item) => item.category === category.name).map(componentCard).join("")}</div></section>`).join("");
  return `<article class="page">${pageHeader("65 public constructors", "Components", "Browse the current public UI surface. Each entry links to source, minimum usage, repository coverage, and native visual evidence when one exists.")}<div class="coverage-legend" aria-label="Coverage legend"><span><i class="coverage-dot coverage-dot--playground"></i>In playground · 32</span><span><i class="coverage-dot coverage-dot--example"></i>Runnable example · 11</span><span><i class="coverage-dot coverage-dot--tests"></i>Test coverage only · 22</span></div>${sections}</article>`;
}

function renderComponent(component) {
  const index = components.indexOf(component);
  const previous = components[index - 1];
  const next = components[index + 1];
  const previewEvidence = component.slug === "button" ? "Live browser render with pointer input" : component.preview ? "Real 2× offscreen Sigil render" : "Not authored yet";
  return `<article class="page component-page"><nav class="breadcrumbs" aria-label="Breadcrumb"><a href="#/components">Components</a><span>/</span><span>${component.category}</span></nav><header class="component-head"><div><p class="eyebrow">${component.category}</p><h1>${component.name}</h1><p>${component.description}</p></div><a class="action" href="${component.source}">Source ${icon("source")}</a></header>${componentDemo(component)}<section class="doc-section"><h2>Install</h2><p>Add the package and import the public prelude. Sigil is 0.1.0; the API moves between 0.x releases.</p>${commandBlock('sigil-ui = { git = "https://github.com/ThomGardiner/sigil.git", tag = "v0.1.0" }')}</section><section class="doc-section"><h2>Usage</h2><p>The focused constructor call:</p>${commandBlock(component.code)}</section><section class="doc-section"><h2>Repository coverage</h2><dl class="coverage-table"><div><dt>Current seat</dt><dd><span class="coverage-badge coverage-badge--${component.coverage}">${coverageLabels[component.coverage]}</span></dd></div><div><dt>Implementation</dt><dd><a href="${component.source}"><code>${component.file}</code> ${icon("source")}</a></dd></div><div><dt>Documentation preview</dt><dd>${previewEvidence}</dd></div>${component.note ? `<div><dt>Feature note</dt><dd>${component.note}</dd></div>` : ""}</dl></section><nav class="page-turn" aria-label="Adjacent components">${previous ? `<a href="#/components/${previous.slug}"><small>Previous</small><span>← ${previous.name}</span></a>` : "<span></span>"}${next ? `<a href="#/components/${next.slug}"><small>Next</small><span>${next.name} →</span></a>` : ""}</nav></article>`;
}

function renderPatterns() {
  const cards = patterns.map((pattern, index) => `<article class="pattern-card"><div class="pattern-card__copy"><span>0${index + 1}</span><h2>${pattern.name}</h2><p>${pattern.description}</p><ul>${pattern.components.map((slug) => { const item = components.find((component) => component.slug === slug); return `<li><a href="#/components/${slug}">${item.name} ${icon("arrow")}</a></li>`; }).join("")}</ul></div></article>`).join("");
  return `<article class="page">${pageHeader("Composed interfaces", "Patterns", "Patterns combine public components into application structures. They are starting points, not a second component API.")}<div class="pattern-list">${cards}</div></article>`;
}

function renderApps() {
  const previews = showcases.map((showcase) => blockPreview(showcase, true)).join("");
  const rows = apps.map((item) => `<article class="app-row" role="row"><div role="cell"><small>${item.kind}</small><h2>${item.name}</h2><a href="${item.source}">Source ${icon("source")}</a></div><p role="cell">${item.description}</p><div role="cell">${commandBlock(item.command, "")}</div></article>`).join("");
  return `<article class="page">${pageHeader("28 runnable packages", "Apps & examples", "Full-window captures are reserved for composed applications. Each image below comes from the checked-in native executable at the stated viewport.")}<section class="block-grid">${previews}</section><div class="app-table" role="table" aria-label="Runnable examples"><div class="app-table__head" role="row"><span role="columnheader">Example</span><span role="columnheader">Purpose</span><span role="columnheader">Run</span></div>${rows}</div></article>`;
}

function renderPerformance() {
  const cards = apps.filter((item) => item.kind === "Performance").map((item) => `<article><small>${item.slug === "access-log" ? "1,000,000 rows" : "10,000 rows · 24 columns"}</small><h2>${item.name}</h2><p>${item.description}</p>${commandBlock(item.command)}</article>`).join("");
  return `<article class="page">${pageHeader("Evidence before numbers", "Performance", "Sigil includes parity and stress examples, but this site does not publish a speed claim without an output-equivalence assertion and a repeatable measurement record.")}<section class="performance-note"><div><p class="eyebrow">Current evidence</p><h2>Two runnable workloads.</h2></div><p>Run them in release mode on the same host. Record frame times only after you confirm equivalent visible output, data, window geometry, and interaction state.</p></section><div class="performance-cards">${cards}</div><section class="method-section"><h2>Minimum comparison record</h2><ol><li><span>01</span><div><h3>Fix the workload</h3><p>Use the checked-in seed, update cadence, viewport, build mode, and window geometry.</p></div></li><li><span>02</span><div><h3>Assert equivalent output</h3><p>Capture both programs and compare the same visible state at a stated pixel-diff threshold.</p></div></li><li><span>03</span><div><h3>Publish the run</h3><p>Include hardware, operating system, commit, sample count, warmup, frame-time distribution, and capture result.</p></div></li></ol></section></article>`;
}

const notFound = () => `<article class="page not-found">${pageHeader("404", "That page is not in the catalogue.", "Use search or return to the component index.")}<a class="action action--primary" href="#/components">Browse components ${icon("arrow")}</a></article>`;

function renderRoute() {
  const [section = "docs", slug] = (location.hash.slice(1) || "/docs").split("/").filter(Boolean);
  let html;
  let title;
  if (section === "docs") [html, title] = [renderDocs(), "Sigil UI · native components for Rust"];
  else if (section === "components" && slug) { const item = components.find((component) => component.slug === slug); [html, title] = item ? [renderComponent(item), `${item.name} · Sigil UI`] : [notFound(), "Not found · Sigil UI"]; }
  else if (section === "components") [html, title] = [renderComponents(), "Components · Sigil UI"];
  else if (section === "patterns") [html, title] = [renderPatterns(), "Patterns · Sigil UI"];
  else if (section === "apps") [html, title] = [renderApps(), "Apps · Sigil UI"];
  else if (section === "performance") [html, title] = [renderPerformance(), "Performance · Sigil UI"];
  else [html, title] = [notFound(), "Not found · Sigil UI"];
  routeView.innerHTML = html;
  document.title = title;
  document.querySelectorAll("[data-nav]").forEach((link) => link.setAttribute("aria-current", link.dataset.nav === section ? "page" : "false"));
  document.querySelectorAll("[data-component-link]").forEach((link) => link.setAttribute("aria-current", link.dataset.componentLink === slug ? "page" : "false"));
  document.querySelectorAll("[data-copy]").forEach((button) => button.addEventListener("click", () => copyText(button)));
  document.querySelectorAll("[data-demo-tab]").forEach((button) => button.addEventListener("click", () => selectDemoTab(button)));
  document.querySelectorAll("[data-stage-label]").forEach((label) => { const img = label.closest(".demo-toolbar")?.nextElementSibling?.querySelector("img"); if (!img) return; const setLabel = () => { label.textContent = `${img.naturalWidth / 2} × ${img.naturalHeight / 2} · 2× capture`; img.width = img.naturalWidth / 2; img.height = img.naturalHeight / 2; }; if (img.complete && img.naturalWidth > 0) setLabel(); else img.addEventListener("load", setLabel); });
  document.querySelectorAll("[data-button-lab]").forEach(initButtonLab);
  window.scrollTo({ top: 0, behavior: "auto" });
}

function buttonLabState(lab) {
  const form = lab.querySelector("form");
  return {
    label: form.elements.label.value.slice(0, 24),
    variant: form.elements.variant.value,
    width: form.elements.width.value,
    disabled: form.elements.disabled.checked,
  };
}

function sendButtonState(lab) {
  const frame = lab.querySelector("iframe");
  const width = lab.querySelector('input[name="width"]');
  const available = Math.max(96, Math.min(280, Math.floor((frame.clientWidth - 48) / 4) * 4));
  width.max = String(available);
  if (Number(width.value) > available) width.value = String(available);
  const state = buttonLabState(lab);
  for (const [name, value] of Object.entries(state)) frame.contentWindow?.postMessage(`sigil-demo:${name}:${value}`, location.origin);
  lab.querySelector("output").value = `${state.width} px`;
  const snippet = buttonSnippet(state);
  lab.querySelector("[data-button-code]").textContent = snippet;
  lab.querySelector("[data-button-code-copy]").dataset.copy = snippet;
}

function initButtonLab(lab) {
  const form = lab.querySelector("form");
  const frame = lab.querySelector("iframe");
  form.addEventListener("input", () => sendButtonState(lab));
  form.addEventListener("change", () => sendButtonState(lab));
  form.addEventListener("reset", () => window.setTimeout(() => sendButtonState(lab)));
  frame.addEventListener("load", () => sendButtonState(lab));
  new ResizeObserver(() => sendButtonState(lab)).observe(frame);
  sendButtonState(lab);
}

function selectDemoTab(button) {
  const demo = button.closest(".component-demo");
  const selected = button.dataset.demoTab;
  demo.querySelectorAll("[data-demo-tab]").forEach((tab) => tab.setAttribute("aria-selected", String(tab === button)));
  demo.querySelectorAll("[data-demo-panel]").forEach((panel) => { panel.hidden = panel.dataset.demoPanel !== selected; });
}

async function copyText(button) {
  try { await navigator.clipboard.writeText(button.dataset.copy); copyNotice.textContent = "Copied to clipboard"; button.dataset.copied = "true"; window.setTimeout(() => { copyNotice.textContent = ""; delete button.dataset.copied; }, 1800); }
  catch { copyNotice.textContent = "Clipboard access failed"; }
}

const searchItems = [
  { name: "Documentation", group: "Page", href: "#/docs", search: "docs getting started install quickstart" },
  { name: "All components", group: "Page", href: "#/components", search: "component inventory catalog" },
  { name: "Patterns", group: "Page", href: "#/patterns", search: "patterns blocks composition" },
  { name: "Apps & examples", group: "Page", href: "#/apps", search: "apps examples runnable packages" },
  { name: "Performance", group: "Page", href: "#/performance", search: "performance benchmark rows methodology" },
  ...components.map((item) => ({ name: item.name, group: item.category, href: `#/components/${item.slug}`, search: `${item.name} ${item.slug} ${item.category} ${item.description}`.toLowerCase() })),
];

const resultLinks = () => [...searchResults.querySelectorAll("a")];
function selectResult(index) { const links = resultLinks(); if (!links.length) return; activeResult = (index + links.length) % links.length; links.forEach((link, position) => link.setAttribute("aria-selected", String(position === activeResult))); links[activeResult].scrollIntoView({ block: "nearest" }); }
function renderSearch(query = "") { const needle = query.trim().toLowerCase(); const matches = searchItems.filter((item) => !needle || `${item.name} ${item.search}`.toLowerCase().includes(needle)).slice(0, 14); activeResult = 0; searchResults.innerHTML = matches.length ? matches.map((item, index) => `<a href="${item.href}" role="option" aria-selected="${index === 0}"><span><strong>${item.name}</strong><small>${item.group}</small></span>${icon("arrow")}</a>`).join("") : '<p class="search-empty">No documentation matches that term.</p>'; resultLinks().forEach((link) => link.addEventListener("click", () => searchDialog.close())); }
function openSearch() { searchInput.value = ""; renderSearch(); searchDialog.showModal(); searchInput.focus(); }

const catalogue = catalogueMarkup();
$("[data-catalog-nav]").innerHTML = catalogue;
$("[data-mobile-catalog]").innerHTML = catalogue;
document.querySelectorAll("[data-open-search]").forEach((button) => button.addEventListener("click", openSearch));
$("[data-open-catalog]").addEventListener("click", () => catalogDialog.showModal());
$("[data-close-catalog]").addEventListener("click", () => catalogDialog.close());
$("[data-mobile-catalog]").addEventListener("click", (event) => { if (event.target.closest("a")) catalogDialog.close(); });
searchInput.addEventListener("input", () => renderSearch(searchInput.value));
searchDialog.addEventListener("keydown", (event) => { if (event.key === "ArrowDown") { event.preventDefault(); selectResult(activeResult + 1); } if (event.key === "ArrowUp") { event.preventDefault(); selectResult(activeResult - 1); } if (event.key === "Enter" && event.target === searchInput) { event.preventDefault(); resultLinks()[activeResult]?.click(); } });
for (const dialog of [searchDialog, catalogDialog]) dialog.addEventListener("click", (event) => { if (event.target === dialog) dialog.close(); });
document.addEventListener("keydown", (event) => { if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") { event.preventDefault(); searchDialog.open ? searchDialog.close() : openSearch(); } });
if (!navigator.platform.toLowerCase().includes("mac")) $(".command-key").textContent = "Ctrl+";
window.addEventListener("hashchange", renderRoute);
renderRoute();
