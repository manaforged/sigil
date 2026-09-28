#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const policy = JSON.parse(readFileSync(join(root, "scripts/gate-policy.json"), "utf8"));
const files = execFileSync("git", ["ls-files", "-z", "--cached", "--others", "--exclude-standard", "*.rs"], {
  cwd: root,
  encoding: "utf8",
})
  .split("\0")
  .filter(Boolean);

const isTestPath = (file) =>
  /(^|\/)(tests|benches|examples|fuzz)\//.test(file) || /(_tests|\/tests)\.rs$/.test(file) || file.startsWith("fuzz/");

function comments(text) {
  const found = [];
  let line = 1;
  let i = 0;
  const at = (k) => text[i + k] ?? "";
  while (i < text.length) {
    const ch = text[i];
    if (ch === "\n") {
      line += 1;
      i += 1;
    } else if (ch === "/" && at(1) === "/") {
      const end = text.indexOf("\n", i);
      found.push([line, text.slice(i, end < 0 ? undefined : end)]);
      i = end < 0 ? text.length : end;
    } else if (ch === "/" && at(1) === "*") {
      const end = text.indexOf("*/", i + 2);
      const body = text.slice(i, end < 0 ? undefined : end + 2);
      found.push([line, body.split("\n")[0]]);
      line += body.split("\n").length - 1;
      i = end < 0 ? text.length : end + 2;
    } else if ((ch === "r" || (ch === "b" && at(1) === "r")) && /^b?r#*"/.test(text.slice(i, i + 12)) && !/\w/.test(text[i - 1] ?? "")) {
      const open = text.slice(i).match(/^b?r(#*)"/);
      const close = `"${open[1]}`;
      const end = text.indexOf(close, i + open[0].length);
      const body = text.slice(i, end < 0 ? undefined : end + close.length);
      line += body.split("\n").length - 1;
      i = end < 0 ? text.length : end + close.length;
    } else if (ch === '"') {
      let j = i + 1;
      while (j < text.length && text[j] !== '"') j += text[j] === "\\" ? 2 : 1;
      line += text.slice(i, j).split("\n").length - 1;
      i = j + 1;
    } else if (ch === "'") {
      const char = text.slice(i).match(/^'(\\u\{[0-9a-fA-F]+\}|\\x[0-9a-fA-F]{2}|\\.|[^\\'\n])'/u);
      i += char ? char[0].length : 1;
    } else {
      i += 1;
    }
  }
  return found;
}

const waiverPattern = (lint) =>
  new RegExp(`#!?\\[(?:cfg_attr\\([^\\]]*?,\\s*)?(?:allow|expect)\\([^\\]]*\\b${lint.replace(/:/g, "\\:")}\\b`, "g");

const problems = [];
const counts = Object.fromEntries(Object.keys(policy.ratchet).map((key) => [key, 0]));
const manual = [];

for (const file of files) {
  const text = readFileSync(join(root, file), "utf8");
  const lines = text.split("\n").length - (text.endsWith("\n") ? 1 : 0);
  if (lines > policy.maxFileLines) problems.push(`${file}: ${lines} lines, cap is ${policy.maxFileLines}`);
  for (const [line, body] of comments(text)) {
    if (!policy.allowedComments.some((prefix) => body.startsWith(prefix))) {
      problems.push(`${file}:${line}: comment: ${body.slice(0, 80)}`);
    }
  }
  for (const match of text.matchAll(/#\[ignore\s*=\s*"((?:[^"\\]|\\[\s\S])*)"\][\s\S]*?\bfn\s+(\w+)/g)) {
    const reason = match[1];
    const tier = Object.entries(policy.ignoreTiers).find(([, prefix]) => reason.startsWith(prefix));
    const line = text.slice(0, match.index).split("\n").length;
    if (!tier) {
      problems.push(`${file}:${line}: #[ignore] reason must start with one of ${Object.values(policy.ignoreTiers).map((p) => JSON.stringify(p)).join(", ")}`);
    } else if (tier[0] === "manual") {
      manual.push(match[2]);
    }
  }
  for (const match of text.matchAll(/#\[test\]\s*(?:#\[[^\]]*\]\s*)*fn\s+(\w+)\s*\(\s*\)\s*\{\s*\}/g)) {
    const line = text.slice(0, match.index).split("\n").length;
    problems.push(`${file}:${line}: test ${match[1]} has an empty body`);
  }
  if (isTestPath(file) || !file.includes("/src/")) continue;
  for (const [key, rule] of Object.entries(policy.ratchet)) {
    const pattern = rule.lint ? waiverPattern(rule.lint) : new RegExp(rule.pattern, "g");
    for (const match of text.matchAll(pattern)) {
      const before = text.slice(0, match.index).trimEnd().split("\n").pop().trim();
      if (before === "#[cfg(test)]") continue;
      counts[key] += 1;
      if (process.argv.includes("--list")) {
        console.log(`${key}: ${file}:${text.slice(0, match.index).split("\n").length}`);
      }
    }
  }
}

if (process.argv.includes("--manual-tests")) {
  process.stdout.write(manual.sort().join("\n") + (manual.length ? "\n" : ""));
  process.exit(0);
}

for (const [key, rule] of Object.entries(policy.ratchet)) {
  if (counts[key] > rule.max) {
    problems.push(`ratchet ${key}: ${counts[key]} in non-test src, cap is ${rule.max}; remove the new one`);
  } else if (counts[key] < rule.max) {
    problems.push(`ratchet ${key}: ${counts[key]} in non-test src, cap is ${rule.max}; lower the cap in scripts/gate-policy.json`);
  }
}

for (const problem of problems) console.log(problem);
console.log(`check-source: ${files.length} files, ${problems.length} problems, ${manual.length} manual tests`);
process.exitCode = problems.length === 0 ? 0 : 1;
