/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * Checks that every translation key used by the UI exists in `en.json`, that
 * every other locale translates it, and that no key is left unused.
 *
 * Understood reference forms:
 *   - t("some.key")                    a literal key
 *   - t(cond ? "a.key" : "b.key")      every literal inside the t() call
 *   - t(`prefix.${value}`)             the whole `prefix.` subtree
 *   - labelKey: "some.key"             a key held in a data table
 */

import fs from "node:fs";
import path from "node:path";

const localeDir = "src/i18n/locales";

/** Flattens a nested locale object into dotted keys. */
function flatten(value, prefix = "") {
  if (typeof value !== "object" || value === null) {
    return [prefix];
  }
  return Object.entries(value).flatMap(([key, child]) =>
    flatten(child, prefix ? `${prefix}.${key}` : key),
  );
}

const resources = Object.fromEntries(
  fs
    .readdirSync(localeDir)
    .filter((name) => name.endsWith(".json"))
    .map((name) => [
      name.replace(/\.json$/, ""),
      new Set(flatten(JSON.parse(fs.readFileSync(path.join(localeDir, name), "utf8")))),
    ]),
);

const english = resources.en;

const files = [];
function walk(dir) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      walk(full);
    } else if (/\.(ts|tsx)$/.test(entry.name) && !/\.test\./.test(entry.name)) {
      files.push(full);
    }
  }
}
walk("src");

const exact = new Map();
const prefixes = new Map();

function note(map, key, file) {
  if (!map.has(key)) {
    map.set(key, []);
  }
  map.get(key).push(file);
}

/** Returns the text of the call starting at `open` (index of `(`). */
function callBody(source, open) {
  let depth = 0;
  for (let index = open; index < source.length; index += 1) {
    const char = source[index];
    if (char === "(") {
      depth += 1;
    } else if (char === ")") {
      depth -= 1;
      if (depth === 0) {
        return source.slice(open + 1, index);
      }
    }
  }
  return "";
}

for (const file of files) {
  const source = fs.readFileSync(file, "utf8");

  // Every `t(...)` call: collect all string literals in its arguments, so
  // ternaries and multi-line calls are covered too. Only dotted literals can be
  // keys, which keeps values like `mode === "setup"` out of the scan.
  for (const match of source.matchAll(/\bt\(/g)) {
    const body = callBody(source, match.index + 1);
    for (const literal of body.matchAll(/["']([\w.]+)["']/g)) {
      if (literal[1].includes(".")) {
        note(exact, literal[1], file);
      }
    }
    for (const dynamic of body.matchAll(/`([\w.]+)\.\$\{/g)) {
      note(prefixes, `${dynamic[1]}.`, file);
    }
  }

  // Keys held in data tables rather than passed to t() directly.
  for (const match of source.matchAll(/labelKey:\s*["']([\w.]+)["']/g)) {
    note(exact, match[1], file);
  }

  // Codes are mapped to `errors.<camelCase>` in one helper rather than t().
  if (source.includes("`errors.${")) {
    note(prefixes, "errors.", file);
  }
}

const problems = [];

for (const [key, where] of exact) {
  if (!english.has(key)) {
    problems.push(`MISSING ${key}  (${where[0]})`);
  }
  for (const [locale, keys] of Object.entries(resources)) {
    if (locale !== "en" && !keys.has(key)) {
      problems.push(`UNTRANSLATED[${locale}] ${key}  (${where[0]})`);
    }
  }
}

for (const prefix of prefixes.keys()) {
  if (![...english].some((key) => key.startsWith(prefix))) {
    problems.push(`MISSING PREFIX ${prefix}`);
  }
}

const isUsed = (key) =>
  exact.has(key) || [...prefixes.keys()].some((prefix) => key.startsWith(prefix));

for (const key of english) {
  if (!isUsed(key)) {
    problems.push(`UNUSED ${key}`);
  }
}

console.log(problems.length === 0 ? "i18n keys OK" : problems.join("\n"));
process.exitCode = problems.length === 0 ? 0 : 1;
