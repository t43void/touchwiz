// Corpus schema, validation, and (de)serialization shared by the content CLI.
//
// A corpus is a JSON file under `src/corpus/` that the Rust engine embeds at
// compile time. This module is the single source of truth for the corpus shape
// so contributors get fast, friendly feedback before ever touching Rust.

import { readFileSync } from "node:fs";
import { basename } from "node:path";

/** The kind of entries a corpus holds. Must match `engine::corpus::CorpusKind`. */
export type CorpusKind = "words" | "sentences" | "code";

/** A parsed corpus. */
export interface Corpus {
  name: string;
  language: string;
  kind: CorpusKind;
  entries: string[];
}

/** The result of validating a corpus: blocking errors and non-blocking warnings. */
export interface ValidationResult {
  errors: string[];
  warnings: string[];
}

const KINDS: readonly CorpusKind[] = ["words", "sentences", "code"];

/**
 * Validates an arbitrary parsed JSON value as a corpus. When `expectedName` is
 * given (the file's stem), the corpus `name` must match it — this keeps the
 * embedded asset name and the on-disk filename in sync.
 */
export function validateCorpus(data: unknown, expectedName?: string): ValidationResult {
  const errors: string[] = [];
  const warnings: string[] = [];

  if (typeof data !== "object" || data === null || Array.isArray(data)) {
    return { errors: ["top-level value must be a JSON object"], warnings };
  }
  const obj = data as Record<string, unknown>;

  if (typeof obj.name !== "string" || obj.name.length === 0) {
    errors.push("`name` must be a non-empty string");
  } else if (expectedName !== undefined && obj.name !== expectedName) {
    errors.push(`\`name\` (${obj.name}) must match the file name (${expectedName})`);
  }

  if (typeof obj.language !== "string" || obj.language.length === 0) {
    errors.push("`language` must be a non-empty string (e.g. \"en\")");
  }

  if (typeof obj.kind !== "string" || !KINDS.includes(obj.kind as CorpusKind)) {
    errors.push(`\`kind\` must be one of ${KINDS.join(", ")}`);
  }

  if (!Array.isArray(obj.entries)) {
    errors.push("`entries` must be an array");
    return { errors, warnings };
  }
  if (obj.entries.length === 0) {
    errors.push("`entries` must contain at least one entry");
  }

  const seen = new Set<string>();
  let duplicates = 0;
  obj.entries.forEach((entry, i) => {
    if (typeof entry !== "string") {
      errors.push(`entry ${i} is not a string`);
      return;
    }
    if (entry.trim().length === 0) {
      errors.push(`entry ${i} is empty or whitespace`);
      return;
    }
    if (seen.has(entry)) {
      duplicates += 1;
    }
    seen.add(entry);
    if (obj.kind === "words" && /\s/.test(entry)) {
      warnings.push(`entry ${i} ("${entry}") contains whitespace but kind is "words"`);
    }
  });
  if (duplicates > 0) {
    warnings.push(`${duplicates} duplicate entrie(s)`);
  }

  return { errors, warnings };
}

/** Reads and parses a corpus file, validating it against its filename stem. */
export function loadCorpusFile(path: string): { corpus: Corpus; result: ValidationResult } {
  const raw = readFileSync(path, "utf8");
  let data: unknown;
  try {
    data = JSON.parse(raw);
  } catch (e) {
    return {
      corpus: { name: "", language: "", kind: "words", entries: [] },
      result: { errors: [`invalid JSON: ${(e as Error).message}`], warnings: [] },
    };
  }
  const stem = basename(path).replace(/\.json$/, "");
  const result = validateCorpus(data, stem);
  return { corpus: data as Corpus, result };
}

/** Serializes a corpus to canonical, pretty-printed JSON (trailing newline). */
export function serializeCorpus(corpus: Corpus): string {
  return `${JSON.stringify(corpus, null, 2)}\n`;
}

/**
 * Builds corpus entries from raw text. For `words`, the text is split on
 * whitespace; for `sentences`/`code`, each non-empty line becomes one entry.
 */
export function entriesFromText(text: string, kind: CorpusKind): string[] {
  if (kind === "words") {
    return text.split(/\s+/).filter((w) => w.length > 0);
  }
  return text
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
}
