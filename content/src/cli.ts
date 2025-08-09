#!/usr/bin/env node
// TypeMaster content authoring CLI.
//
// Build/dev-time only — never required at runtime. Corpora authored here are
// embedded into the Rust binary at compile time. Run a subcommand:
//
//   node src/cli.ts validate [file...]   # validate corpora (default: all)
//   node src/cli.ts list                 # list corpora with stats
//   node src/cli.ts add <name> --lang en --kind words --from words.txt
//   node src/cli.ts watch                # re-validate on change (dev)

import { existsSync, readdirSync, readFileSync, watch, writeFileSync } from "node:fs";
import { join } from "node:path";
import { parseArgs } from "node:util";

import {
  type Corpus,
  type CorpusKind,
  entriesFromText,
  loadCorpusFile,
  serializeCorpus,
  validateCorpus,
} from "./corpus.ts";

const CORPUS_DIR = join(import.meta.dirname, "corpus");

function corpusFiles(): string[] {
  return readdirSync(CORPUS_DIR)
    .filter((f) => f.endsWith(".json"))
    .map((f) => join(CORPUS_DIR, f));
}

/** Validates the given files (or all corpora). Returns true if all are valid. */
function validate(files: string[]): boolean {
  const targets = files.length > 0 ? files : corpusFiles();
  let ok = true;
  for (const path of targets) {
    const { result } = loadCorpusFile(path);
    const name = path.split("/").pop();
    if (result.errors.length === 0) {
      const warn = result.warnings.length > 0 ? `  (${result.warnings.length} warning(s))` : "";
      console.log(`  ok    ${name}${warn}`);
      for (const w of result.warnings) console.log(`        warning: ${w}`);
    } else {
      ok = false;
      console.log(`  FAIL  ${name}`);
      for (const e of result.errors) console.log(`        error: ${e}`);
      for (const w of result.warnings) console.log(`        warning: ${w}`);
    }
  }
  return ok;
}

/** Prints a table of all corpora with their kind, language, and entry count. */
function list(): void {
  console.log("  name                 lang  kind        entries  status");
  console.log("  -------------------  ----  ----------  -------  ------");
  for (const path of corpusFiles()) {
    const { corpus, result } = loadCorpusFile(path);
    const status = result.errors.length === 0 ? "ok" : "INVALID";
    const count = Array.isArray(corpus.entries) ? corpus.entries.length : 0;
    console.log(
      `  ${(corpus.name ?? "?").padEnd(19)}  ${(corpus.language ?? "?").padEnd(4)}  ` +
        `${(corpus.kind ?? "?").padEnd(10)}  ${String(count).padStart(7)}  ${status}`,
    );
  }
}

/** Creates a new corpus JSON file from a plain-text source. */
function add(positionals: string[], values: Record<string, unknown>): boolean {
  const name = positionals[0];
  const lang = values.lang as string | undefined;
  const kind = values.kind as CorpusKind | undefined;
  const from = values.from as string | undefined;

  if (!name || !lang || !kind || !from) {
    console.error("usage: add <name> --lang <code> --kind <words|sentences|code> --from <textfile>");
    return false;
  }
  if (!existsSync(from)) {
    console.error(`source file not found: ${from}`);
    return false;
  }

  let entries = entriesFromText(readFileSync(from, "utf8"), kind);
  if (values.lowercase) entries = entries.map((e) => e.toLowerCase());
  if (values.dedupe) entries = [...new Set(entries)];

  const corpus: Corpus = { name, language: lang, kind, entries };
  const result = validateCorpus(corpus, name);
  if (result.errors.length > 0) {
    console.error(`refusing to write invalid corpus:`);
    for (const e of result.errors) console.error(`  error: ${e}`);
    return false;
  }

  const outPath = join(CORPUS_DIR, `${name}.json`);
  writeFileSync(outPath, serializeCorpus(corpus));
  console.log(`wrote ${outPath} (${entries.length} entries)`);
  console.log("remember to rebuild the Rust binary to embed the new corpus.");
  return true;
}

/** Watches the corpus directory and re-validates on every change (dev aid). */
function watchCorpora(): void {
  console.log(`watching ${CORPUS_DIR} — press Ctrl+C to stop`);
  validate([]);
  watch(CORPUS_DIR, { persistent: true }, (_event, filename) => {
    if (filename && filename.endsWith(".json")) {
      console.log(`\n[change] ${filename}`);
      validate([join(CORPUS_DIR, filename)]);
    }
  });
}

function main(): void {
  const [command, ...rest] = process.argv.slice(2);
  switch (command) {
    case "validate": {
      process.exit(validate(rest) ? 0 : 1);
      break;
    }
    case "list": {
      list();
      break;
    }
    case "add": {
      const { positionals, values } = parseArgs({
        args: rest,
        allowPositionals: true,
        options: {
          lang: { type: "string" },
          kind: { type: "string" },
          from: { type: "string" },
          lowercase: { type: "boolean" },
          dedupe: { type: "boolean" },
        },
      });
      process.exit(add(positionals, values) ? 0 : 1);
      break;
    }
    case "watch": {
      watchCorpora();
      break;
    }
    default: {
      console.error("commands: validate [file...] | list | add <name> ... | watch");
      process.exit(1);
    }
  }
}

main();
