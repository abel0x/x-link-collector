// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 abel0x <https://github.com/abel0x>

/**
 * The two languages stay in step: English is the source, Turkish carries
 * every key English has, with the same {placeholders}, and nothing the pages
 * ask for is missing.
 *
 *   node --test tests/        (or: make test-ext)
 */

import { readFileSync } from 'node:fs';
import { createContext, runInContext } from 'node:vm';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import assert from 'node:assert/strict';
import test from 'node:test';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const read = (path) => readFileSync(join(root, path), 'utf8');
const json = (path) => JSON.parse(read(path));

const panel = { en: json('receiver/panel/i18n/en.json'), tr: json('receiver/panel/i18n/tr.json') };

/** `{n}` style names in a string, or in every form of a plural. */
function holes(text) {
  const forms = Array.isArray(text) ? text : [text];
  return [...new Set(forms.flatMap((f) => f.match(/\{\w+\}/g) || []))].sort();
}

function sameShape(name, en, tr) {
  assert.deepEqual(Object.keys(tr).sort(), Object.keys(en).sort(), `${name}: the same keys in both`);
  for (const [key, text] of Object.entries(en)) {
    const other = tr[key];
    for (const form of [text, other].flat()) {
      assert.equal(typeof form, 'string', `${name} ${key}: text`);
      assert.ok(form.trim(), `${name} ${key}: not empty`);
    }
    if (Array.isArray(text)) assert.equal(text.length, 2, `${name} ${key}: singular and plural`);
    if (Array.isArray(other)) assert.equal(other.length, 2, `${name} ${key}: two forms`);
    assert.deepEqual(holes(other), holes(text), `${name} ${key}: the same {placeholders}`);
  }
}

test('the panel: Turkish has every English key, with the same placeholders', () => {
  sameShape('panel', panel.en, panel.tr);
});

test('every key the panel names is there', () => {
  const html = read('receiver/panel/index.html');
  const script = read('receiver/panel/panel.js');
  const named = new Set([
    ...[...html.matchAll(/data-t(?:-placeholder|-label)?="([^"]+)"/g)].map((m) => m[1]),
    ...[...script.matchAll(/\bt\('([\w.-]+)'/g)].map((m) => m[1]),
    ...[...script.matchAll(/'(why\.\w+)'/g)].map((m) => m[1]),
  ]);
  assert.ok(named.size > 100, 'found the references');
  for (const key of named) assert.ok(key in panel.en, `missing word: ${key}`);
});

test('the panel opens in English unless the browser asks for Turkish', () => {
  const script = read('receiver/panel/panel.js');
  assert.match(script, /startsWith\('tr'\) \? 'tr' : 'en'/);
  assert.match(read('receiver/panel/index.html'), /<html lang="en">/);
});

test('the extension: _locales in both languages, English the default', () => {
  const manifest = json('extension/manifest.json');
  assert.equal(manifest.default_locale, 'en');
  const en = json('extension/_locales/en/messages.json');
  const tr = json('extension/_locales/tr/messages.json');
  const flat = (all) => Object.fromEntries(Object.entries(all).map(([k, v]) => [k, v.message]));
  sameShape('_locales', flat(en), flat(tr));
  for (const key of Object.keys(en)) {
    assert.deepEqual(Object.keys(tr[key].placeholders || {}), Object.keys(en[key].placeholders || {}), key);
  }
});

test("the extension's own page: both languages", () => {
  const source = read('extension/options.js');
  const start = source.indexOf('const WORDS = {');
  const end = source.indexOf('\n};', start) + 3;
  const context = createContext({});
  runInContext(`${source.slice(start, end)}\nthis.WORDS = WORDS;`, context);
  sameShape('options', context.WORDS.en, context.WORDS.tr);
});
