import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const lib = readFileSync(join(root, "crates/sigil/src/lib.rs"), "utf8")
  .replace(/#!?\[[^\]]*\]/g, "");
const doc = readFileSync(join(root, "docs/api.md"), "utf8");

function splitPrelude(source) {
  const start = source.indexOf("pub mod prelude {");
  let depth = 0;
  for (let i = source.indexOf("{", start); i < source.length; i += 1) {
    if (source[i] === "{") depth += 1;
    if (source[i] === "}") depth -= 1;
    if (depth === 0) {
      return [`${source.slice(0, start)}pub mod prelude;${source.slice(i + 1)}`, source.slice(start, i + 1)];
    }
  }
  throw new Error("lib.rs: unterminated prelude");
}

function leaves(tree) {
  const open = tree.indexOf("{");
  if (open < 0) {
    const parts = tree.trim().split(/\s+as\s+/);
    return [parts.length > 1 ? parts[1] : parts[0].split("::").pop()];
  }
  const body = tree.slice(open + 1, tree.lastIndexOf("}"));
  const items = [];
  let depth = 0;
  let current = "";
  for (const ch of body) {
    if (ch === "{") depth += 1;
    if (ch === "}") depth -= 1;
    if (ch === "," && depth === 0) {
      items.push(current);
      current = "";
    } else {
      current += ch;
    }
  }
  items.push(current);
  return items.filter((item) => item.trim()).flatMap(leaves);
}

function exports(source, withItems) {
  const names = new Set();
  for (const match of source.matchAll(/pub use ([^;]+);/g)) {
    for (const name of leaves(match[1])) names.add(name.trim());
  }
  if (withItems) {
    for (const match of source.matchAll(/pub (?:mod|fn) (\w+)/g)) names.add(match[1]);
  }
  return names;
}

function documented(heading) {
  const start = doc.indexOf(`\n## ${heading}\n`);
  if (start < 0) throw new Error(`api.md: missing section "## ${heading}"`);
  const end = doc.indexOf("\n## ", start + 1);
  const names = new Set();
  for (const line of doc.slice(start, end < 0 ? undefined : end).split("\n")) {
    const cell = line.match(/^\| `([\w!]+)` \|/);
    if (cell) names.add(cell[1]);
  }
  return names;
}

const [rootSource, preludeSource] = splitPrelude(lib);
const pairs = [
  ["Crate root", exports(rootSource, true)],
  ["Prelude", exports(preludeSource, false)],
];
let differences = 0;
for (const [heading, exported] of pairs) {
  const listed = documented(heading);
  for (const name of [...exported].sort()) {
    if (!listed.has(name)) {
      console.log(`${heading}: exported, not in api.md: ${name}`);
      differences += 1;
    }
  }
  for (const name of [...listed].sort()) {
    if (!exported.has(name)) {
      console.log(`${heading}: in api.md, not exported: ${name}`);
      differences += 1;
    }
  }
}
process.exitCode = differences === 0 ? 0 : 1;
