#!/usr/bin/env node

import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const policy = JSON.parse(readFileSync(join(root, "scripts/gate-policy.json"), "utf8"));
const budgets = policy.story.budgets;
const ignored = new Set(policy.story.ignored);

function api(crate, file) {
  const found = new Set([crate]);
  const paths = new RegExp(`\\b${crate}(?:::[A-Za-z_]\\w*)+`, "g");
  for (const path of readFileSync(join(root, file), "utf8").matchAll(paths)) {
    for (const part of path[0].split("::").slice(1)) found.add(part);
  }
  return found;
}

const names = api("sigil", "api/sigil-ui.txt");
const harness = new Set([...names, ...api("sigil_kittest", "api/sigil-kittest.txt")]);

function strip(source) {
  return source
    .replace(/\/\/[^\n]*/g, "")
    .replace(/#!?\[[^\]]*\]/g, "")
    .replace(/(?<![\w])r(#*)"[\s\S]*?"\1/g, '""')
    .replace(/"(?:\\.|[^"\\])*"/g, '""')
    .replace(/'(?:\\.|[^'\\])'/g, "''");
}

function declared(source) {
  const found = new Set();
  const add = (pattern, group = 1) => {
    for (const match of source.matchAll(pattern)) found.add(match[group]);
  };
  add(/\blet\s+(?:mut\s+)?(\w+)/g);
  add(/\b[A-Z]\w*\(\s*(?:mut\s+)?([a-z_]\w*)\s*\)(?:\s+if\s[^\n]*?)?\s*=>/g);
  add(/\blet\s+(?:\w+::)*[A-Z]\w*\(\s*(?:mut\s+)?([a-z_]\w*)\s*\)\s*=/g);
  add(/\b(?:fn|struct|enum|mod)\s+(\w+)/g);
  for (const params of source.matchAll(/\bfn\s+\w+\s*(?:<[^>]*>)?\(([^)]*)\)/g)) {
    for (const match of params[1].matchAll(/(\w+)\s*:/g)) found.add(match[1]);
  }
  for (const body of source.matchAll(/\bstruct\s+\w+\s*\{([^}]*)\}/g)) {
    for (const match of body[1].matchAll(/(\w+)\s*:/g)) found.add(match[1]);
  }
  for (const params of source.matchAll(/\|([^|]*)\|/g)) {
    if (/^[\w\s,:&'()<>]*$/.test(params[1])) {
      for (const match of params[1].matchAll(/(?<![:\w])(\w+)(?=\s*(?:[,:)]|$))/g)) found.add(match[1]);
    }
  }
  return found;
}

function count(file) {
  const source = strip(readFileSync(file, "utf8"));
  const own = declared(source);
  const known = source.includes("sigil_kittest") ? harness : names;
  const used = new Set();
  for (const match of source.matchAll(/\b[A-Za-z_]\w*\b/g)) {
    const word = match[0];
    if (known.has(word) && !ignored.has(word) && !own.has(word)) used.add(word);
  }
  return [...used].sort();
}

const dir = join(root, "crates/sigil/examples");
const stories = readdirSync(dir).filter((file) => /^story-.*\.rs$/.test(file));
const problems = [];
for (const file of stories) {
  const story = file.replace(/\.rs$/, "");
  const used = count(join(dir, file));
  const budget = budgets[story];
  if (budget === undefined) {
    problems.push(`${story}: no budget in scripts/gate-policy.json story.budgets (uses ${used.length})`);
  } else if (used.length > budget) {
    problems.push(`${story}: ${used.length} names, budget is ${budget}: ${used.join(" ")}`);
  } else if (used.length < budget) {
    problems.push(`${story}: ${used.length} names, budget is ${budget}; lower the budget in scripts/gate-policy.json`);
  }
}
const prelude = new Set(
  [...readFileSync(join(root, "api/sigil-ui.txt"), "utf8").matchAll(/\bsigil::prelude::([A-Za-z_]\w*)/g)].map((match) => match[1]),
);
if (prelude.size > policy.story.prelude) {
  problems.push(`prelude: ${prelude.size} names, budget is ${policy.story.prelude}`);
} else if (prelude.size < policy.story.prelude) {
  problems.push(`prelude: ${prelude.size} names, budget is ${policy.story.prelude}; lower the budget in scripts/gate-policy.json`);
}
for (const story of Object.keys(budgets)) {
  if (!stories.includes(`${story}.rs`)) problems.push(`${story}: budget without a story file`);
}
for (const problem of problems) console.log(problem);
console.log(`check-story: ${stories.length} stories, ${problems.length} problems`);
process.exit(problems.length === 0 ? 0 : 1);
