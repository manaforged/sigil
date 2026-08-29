#!/usr/bin/env node

import { access, readFile, readdir } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { apps, categories, components, patterns, showcases } from "../site/catalog.mjs";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const failures = [];
const expect = (condition, message) => { if (!condition) failures.push(message); };
const exists = async (path) => access(join(root, path)).then(() => true, () => false);

async function rustFiles(path) {
  const files = [];
  for (const entry of await readdir(join(root, path), { withFileTypes: true })) {
    const relative = join(path, entry.name);
    if (entry.isDirectory()) files.push(...await rustFiles(relative));
    else if (entry.name.endsWith(".rs")) files.push(relative);
  }
  return files;
}

async function rustSource(path) {
  const files = await rustFiles(path);
  return (await Promise.all(files.map((file) => readFile(join(root, file), "utf8")))).join("\n");
}

expect(components.length === 65, `expected 65 components, found ${components.length}`);
expect(apps.length === 28, `expected 28 apps, found ${apps.length}`);
expect(patterns.length === 5, `expected 5 patterns, found ${patterns.length}`);
expect(showcases.length === 3, `expected 3 application showcases, found ${showcases.length}`);
expect(new Set(components.map(({ slug }) => slug)).size === components.length, "component slugs must be unique");
expect(new Set(apps.map(({ slug }) => slug)).size === apps.length, "app slugs must be unique");
expect(categories.reduce((sum, category) => sum + category.count, 0) === components.length, "category counts must equal the component total");
expect(components.filter(({ coverage }) => coverage === "playground").length === 32, "expected 32 playground components");
expect(components.filter(({ coverage }) => coverage === "example").length === 13, "expected 13 example-only components");
expect(components.filter(({ coverage }) => coverage === "tests").length === 20, "expected 20 test-only components");

for (const category of categories) {
  const actual = components.filter((component) => component.category === category.name).length;
  expect(actual === category.count, `${category.name} declares ${category.count} components but contains ${actual}`);
}

for (const component of components) {
  expect(await exists(component.file), `${component.slug}: missing source ${component.file}`);
  if (await exists(component.file)) {
    const source = await readFile(join(root, component.file), "utf8");
    const symbol = component.slug.startsWith("divider-")
      ? component.slug.slice("divider-".length)
      : component.slug.replaceAll("-", "_");
    expect(source.includes(symbol), `${component.slug}: ${component.file} does not mention ${symbol}`);
  }
  if (component.preview) expect(await exists(`site/assets/components/${component.preview}`), `${component.slug}: missing component preview ${component.preview}`);
}

for (const pattern of patterns) {
  for (const slug of pattern.components) expect(components.some((component) => component.slug === slug), `${pattern.slug}: unknown component ${slug}`);
}

for (const showcase of showcases) {
  expect(await exists(`site/assets/screenshots/${showcase.screenshot}`), `${showcase.slug}: missing application screenshot ${showcase.screenshot}`);
  expect(apps.some((app) => app.slug === showcase.slug), `${showcase.slug}: showcase has no runnable app`);
}

for (const app of apps) expect(await exists(app.folder), `${app.slug}: missing example folder ${app.folder}`);

const playgroundSource = await rustSource("examples/widget-playground/src");
const exampleSource = await rustSource("examples");
for (const component of components) {
  const symbol = component.slug.replaceAll("-", "_");
  const constructor = new RegExp(`(?<![.\\w])${symbol}\\s*\\(`);
  const coverage = constructor.test(playgroundSource)
    ? "playground"
    : constructor.test(exampleSource) ? "example" : "tests";
  expect(component.coverage === coverage, `${component.slug}: catalog says ${component.coverage}, source scan says ${coverage}`);
}

const siteFiles = await readdir(join(root, "site"));
for (const required of ["index.html", "site.css", "site.js", "catalog.mjs", "tokens.css", "favicon.svg"]) {
  expect(siteFiles.includes(required), `site is missing ${required}`);
}

const nativeGoldens = (await readdir(join(root, "crates/sigil/tests/goldens"), { recursive: true })).filter((file) => file.endsWith(".png"));
expect(nativeGoldens.length === 116, `site copy says 114 native golden captures, found ${nativeGoldens.length}`);

const html = await readFile(join(root, "site/index.html"), "utf8");
for (const hook of ["data-route-view", "data-catalog-nav", "data-command-dialog", "data-command-results", "data-catalog-dialog"]) {
  expect(html.includes(hook), `index.html is missing ${hook}`);
}

if (failures.length) {
  console.error(`Site catalog check failed (${failures.length}):`);
  for (const failure of failures) console.error(`- ${failure}`);
  process.exitCode = 1;
} else {
  console.log(`Site catalog check passed: ${components.length} components, ${patterns.length} patterns, ${apps.length} apps.`);
}
