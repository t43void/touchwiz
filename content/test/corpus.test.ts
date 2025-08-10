import assert from "node:assert/strict";
import { test } from "node:test";

import { entriesFromText, validateCorpus } from "../src/corpus.ts";

test("a well-formed corpus validates cleanly", () => {
  const corpus = {
    name: "sample",
    language: "en",
    kind: "words",
    entries: ["the", "and", "for"],
  };
  const { errors, warnings } = validateCorpus(corpus, "sample");
  assert.equal(errors.length, 0);
  assert.equal(warnings.length, 0);
});

test("name must match the file stem", () => {
  const corpus = { name: "wrong", language: "en", kind: "words", entries: ["a"] };
  const { errors } = validateCorpus(corpus, "expected");
  assert.ok(errors.some((e) => e.includes("must match the file name")));
});

test("unknown kind is rejected", () => {
  const corpus = { name: "x", language: "en", kind: "paragraphs", entries: ["a"] };
  const { errors } = validateCorpus(corpus, "x");
  assert.ok(errors.some((e) => e.includes("`kind`")));
});

test("empty entries are rejected", () => {
  const corpus = { name: "x", language: "en", kind: "words", entries: [] };
  const { errors } = validateCorpus(corpus, "x");
  assert.ok(errors.some((e) => e.includes("at least one")));
});

test("whitespace in a words entry is a warning, not an error", () => {
  const corpus = { name: "x", language: "en", kind: "words", entries: ["two words"] };
  const { errors, warnings } = validateCorpus(corpus, "x");
  assert.equal(errors.length, 0);
  assert.ok(warnings.some((w) => w.includes("whitespace")));
});

test("duplicates produce a warning", () => {
  const corpus = { name: "x", language: "en", kind: "words", entries: ["a", "a"] };
  const { warnings } = validateCorpus(corpus, "x");
  assert.ok(warnings.some((w) => w.includes("duplicate")));
});

test("non-object input is rejected", () => {
  const { errors } = validateCorpus([1, 2, 3]);
  assert.ok(errors.some((e) => e.includes("must be a JSON object")));
});

test("entriesFromText splits words on whitespace", () => {
  assert.deepEqual(entriesFromText("the  and\nfor ", "words"), ["the", "and", "for"]);
});

test("entriesFromText keeps whole lines for sentences", () => {
  assert.deepEqual(entriesFromText("first line\n\n second line \n", "sentences"), [
    "first line",
    "second line",
  ]);
});
