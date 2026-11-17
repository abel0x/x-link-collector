// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 abel0x <https://github.com/abel0x>

/**
 * Pure helpers from the panel's script, taken out of it and run on their own.
 *
 *   node --test tests/        (or: make test-ext)
 */

import { readFileSync } from 'node:fs';
import { createContext, runInContext } from 'node:vm';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import assert from 'node:assert/strict';
import test from 'node:test';

const source = readFileSync(join(dirname(fileURLToPath(import.meta.url)), '..', 'receiver', 'panel', 'panel.js'), 'utf8');

/** One top-level function of panel.js, by name, in a context of its own. */
function take(name) {
  const start = source.indexOf(`function ${name}(`);
  assert.ok(start >= 0, `${name} is in panel.js`);
  const end = source.indexOf('\n}\n', start) + 2;
  const context = createContext({});
  runInContext(`${source.slice(start, end)}\nthis.fn = ${name};`, context);
  return context.fn;
}

test('an X archive gives the liked tweets, not the ones their text quotes', () => {
  const archiveLinks = take('archiveLinks');
  const likes = `window.YTD.like.part0 = [
    { "like" : { "tweetId" : "1790637656616943991", "fullText" : "see https://x.com/other/status/111",
                 "expandedUrl" : "https://twitter.com/i/web/status/1790637656616943991" } },
    { "like" : { "tweetId" : "1790637656616943992" } },
    { "like" : { "tweetId" : "1790637656616943991" } }
  ]`;
  assert.deepEqual([...archiveLinks(likes)], [
    'https://x.com/i/web/status/1790637656616943991',
    'https://x.com/i/web/status/1790637656616943992',
  ]);
});

test('any other text gives the tweet links in it', () => {
  const archiveLinks = take('archiveLinks');
  const text = 'a https://x.com/alice/status/1 b https://mobile.twitter.com/bob/status/2?s=20 '
    + 'c https://x.com/alice/status/1 d https://example.com/status/3 e https://x.com/home';
  assert.deepEqual([...archiveLinks(text)], ['https://x.com/alice/status/1', 'https://mobile.twitter.com/bob/status/2']);
  assert.deepEqual([...archiveLinks('nothing here')], []);
});
