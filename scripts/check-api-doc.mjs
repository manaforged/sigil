import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const src = join(root, "crates/sigil/src");
const rawLib = readFileSync(join(src, "lib.rs"), "utf8");
const lib = rawLib
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
const manifest = readFileSync(join(root, "crates/sigil/Cargo.toml"), "utf8");
const privateFeatures = new Set(["kittest", "test-clock"]);
const features = new Set(
  [...manifest.slice(manifest.indexOf("[features]")).split(/\n\[/)[0].matchAll(/^([\w-]+)\s*=/gm)]
    .map((match) => match[1])
    .filter((name) => !privateFeatures.has(name)),
);

function cfgArgs(text) {
  const args = [];
  let depth = 0;
  let current = "";
  for (const ch of text) {
    if (ch === "(") depth += 1;
    if (ch === ")") depth -= 1;
    if (ch === "," && depth === 0) {
      args.push(current.trim());
      current = "";
    } else {
      current += ch;
    }
  }
  if (current.trim()) args.push(current.trim());
  return args;
}

function cfgHolds(predicate) {
  const call = predicate.match(/^(all|any|not)\s*\(([\s\S]*)\)$/);
  if (call) {
    const args = cfgArgs(call[2]);
    if (call[1] === "all") return args.every(cfgHolds);
    if (call[1] === "any") return args.some(cfgHolds);
    return !cfgHolds(args[0]);
  }
  const feature = predicate.match(/^feature\s*=\s*"([\w-]+)"$/);
  if (feature) return features.has(feature[1]);
  return !["test", "doctest"].includes(predicate);
}

function attributesHold(attributes) {
  return attributes.every((attribute) => {
    const cfg = attribute.match(/^#\[cfg\(([\s\S]*)\)\]$/);
    return !cfg || cfgHolds(cfg[1].trim());
  });
}

function block(lines, from) {
  let depth = 0;
  for (let i = from; i < lines.length; i += 1) {
    for (const ch of lines[i]) {
      if (ch === "{") depth += 1;
      if (ch === "}") depth -= 1;
    }
    if (depth === 0 && lines[i].includes("}")) return i;
  }
  throw new Error("unterminated block");
}

function publicModule(text, path, dir, out) {
  const lines = text.split("\n");
  let attributes = [];
  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    if (line.startsWith("#[")) {
      let attribute = line;
      while ((attribute.match(/\[/g) || []).length > (attribute.match(/\]/g) || []).length) {
        i += 1;
        attribute += `\n${lines[i]}`;
      }
      attributes.push(attribute.trim());
      continue;
    }
    const holds = attributesHold(attributes);
    attributes = [];
    const inline = line.match(/^pub mod (\w+) \{/);
    const external = line.match(/^pub mod (\w+);/);
    if (inline) {
      const end = block(lines, i);
      if (holds) {
        const body = lines.slice(i + 1, end).map((row) => row.replace(/^ {4}/, "")).join("\n");
        publicModule(body, `${path}::${inline[1]}`, join(dir, inline[1]), out);
      }
      i = end;
      continue;
    }
    if (external) {
      if (holds) {
        const file = [join(dir, `${external[1]}.rs`), join(dir, external[1], "mod.rs")].find(existsSync);
        if (!file) throw new Error(`${path}: missing module file ${external[1]}`);
        const childDir = file.endsWith("mod.rs") ? dirname(file) : join(dir, external[1]);
        publicModule(readFileSync(file, "utf8"), `${path}::${external[1]}`, childDir, out);
      }
      continue;
    }
    if (!holds) continue;
    const reexport = line.match(/^pub use /);
    if (reexport) {
      let statement = line;
      while (!statement.includes(";")) {
        i += 1;
        statement += `\n${lines[i]}`;
      }
      for (const name of leaves(statement.replace(/^pub use /, "").split(";")[0])) {
        const trimmed = name.trim();
        if (trimmed !== "*" && trimmed !== "self") out.push([path, trimmed]);
      }
      continue;
    }
    const item = line.match(
      /^pub (?:(?:const|async|unsafe|extern "C") )*(?:fn|struct|enum|trait|type|const|static|union) (\w+)/,
    );
    if (item) out.push([path, item[1]]);
  }
}

function moduleDifferences() {
  const rows = doc
    .split("\n")
    .filter((line) => line.startsWith("|"))
    .flatMap((line) => [...line.matchAll(/`([^`]+)`/g)].map((match) => match[1]))
    .join("\n");
  const items = [];
  publicModule(rawLib, "sigil", src, items);
  let count = 0;
  const seen = new Set();
  for (const [path, item] of items) {
    const module = path.replace(/^sigil::?/, "");
    const key = `${module}: ${item}`;
    if (!module || module.startsWith("prelude") || seen.has(key)) continue;
    seen.add(key);
    if (process.argv.includes("--count")) continue;
    if (!new RegExp(`\\b${item}\\b`).test(rows)) {
      console.log(`${module}: public, not in api.md: ${item}`);
      count += 1;
    }
  }
  if (process.argv.includes("--count")) console.log(`module items: ${seen.size}`);
  return count;
}

if (process.argv.includes("--modules")) differences += moduleDifferences();
process.exitCode = differences === 0 ? 0 : 1;
