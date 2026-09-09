// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 abel0x <https://github.com/abel0x>

/**
 * Tests for the service worker's URL canonicalisation and tab-closing rules.
 *
 *   node --test extension/test/        (or: make test-ext)
 *
 * background.js is loaded into a fresh V8 context per test with `chrome` and
 * `fetch` stubbed, so each case starts from clean worker state.
 */

import { readFileSync } from 'node:fs';
import { createContext, runInContext } from 'node:vm';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import assert from 'node:assert/strict';
import test from 'node:test';

const here = dirname(fileURLToPath(import.meta.url));
const source = readFileSync(join(here, '..', 'background.js'), 'utf8');

/** Load background.js into an isolated context and return its stubs. */
function loadWorker({ respond = () => ({ ok: true, status: 200 }) } = {}) {
  const posted = [];
  const removed = [];
  const on = {};

  const sandbox = {
    URL,
    Date,
    AbortController,
    Promise,
    console: { info() {}, warn() {}, error() {} },
    // Unref'd so a pending badge timer cannot hold the test process open.
    setTimeout: (fn, ms) => {
      const t = setTimeout(fn, ms);
      t.unref?.();
      return t;
    },
    clearTimeout,
    fetch: async (url, init) => {
      posted.push({ url, body: init?.body, method: init?.method });
      const result = respond(posted.length);
      if (result instanceof Error) throw result;
      return { ...result, json: async () => ({ ok: true }) };
    },
    chrome: {
      tabs: {
        TAB_ID_NONE: -1,
        onCreated: { addListener: (fn) => (on.created = fn) },
        onUpdated: { addListener: (fn) => (on.updated = fn) },
        onRemoved: { addListener: (fn) => (on.removed = fn) },
        remove: async (id) => {
          removed.push(id);
        },
      },
      action: {
        setBadgeText: async () => {},
        setBadgeBackgroundColor: async () => {},
        onClicked: { addListener: (fn) => (on.clicked = fn) },
      },
    },
  };

  createContext(sandbox);
  runInContext(source, sandbox, { filename: 'background.js' });
  return { sandbox, on, posted, removed };
}

/** Let the worker's promise chain run to completion. */
const settle = async () => {
  for (let i = 0; i < 5; i += 1) await new Promise((r) => setTimeout(r, 0));
};

const openTab = async (on, id, url) => {
  on.created({ id, pendingUrl: url });
  await settle();
};

test('canonicalises status URLs and rejects everything else', () => {
  const { sandbox } = loadWorker();
  const clean = sandbox.canonicalTweetUrl;

  for (const dirty of [
    'https://x.com/jack/status/20',
    'http://twitter.com/jack/status/20',
    'https://mobile.twitter.com/jack/status/20?s=20&t=TRACKING',
    'https://www.x.com/jack/status/20/photo/1',
    'https://x.com/jack/status/20#anchor',
    'https://x.com/jack/statuses/20',
  ]) {
    assert.equal(clean(dirty), 'https://x.com/jack/status/20', dirty);
  }

  assert.equal(clean('https://x.com/i/web/status/77'), 'https://x.com/i/web/status/77');
  assert.equal(clean('https://x.com/i/status/77?k=v'), 'https://x.com/i/web/status/77');

  for (const notATweet of [
    'https://x.com/home',
    'https://x.com/jack',
    'https://x.com/search?q=rust',
    'https://x.com/jack/status/notanumber',
    'https://example.com/jack/status/20',
    'https://x.com.evil.tld/jack/status/20',
    'about:blank',
    'chrome://newtab/',
    '',
    null,
    undefined,
  ]) {
    assert.equal(clean(notATweet), null, String(notATweet));
  }
});

test('captures a middle-clicked tweet and closes its tab', async () => {
  const { on, posted, removed } = loadWorker();
  await openTab(on, 7, 'https://x.com/jack/status/20?s=20');

  assert.equal(posted.length, 1);
  assert.equal(posted[0].method, 'POST');
  assert.equal(posted[0].body, 'https://x.com/jack/status/20');
  assert.deepEqual(removed, [7]);
});

test('never closes a tab it did not open', async () => {
  const { on, posted, removed } = loadWorker();
  // The feed tab is a same-tab SPA navigation: no onCreated for this id.
  on.updated(3, { url: 'https://x.com/jack/status/20' }, { id: 3 });
  await settle();

  assert.deepEqual(posted, []);
  assert.deepEqual(removed, []);
});

test('leaves the tab open when the receiver does not confirm', async () => {
  const { on, posted, removed } = loadWorker({
    respond: () => new Error('ECONNREFUSED'),
  });
  await openTab(on, 8, 'https://x.com/jack/status/20');

  assert.equal(posted.length, 1, 'it tried once');
  assert.deepEqual(removed, [], 'tab must survive so the link is not lost');
});

test('backs off instead of retrying every tab while the receiver is down', async () => {
  const { on, posted } = loadWorker({ respond: () => new Error('ECONNREFUSED') });
  await openTab(on, 9, 'https://x.com/a/status/1');
  await openTab(on, 10, 'https://x.com/b/status/2');
  await openTab(on, 11, 'https://x.com/c/status/3');

  assert.equal(posted.length, 1, 'one failure, then cooldown');
});

test('does not close a tab on a non-2xx reply', async () => {
  const { on, removed } = loadWorker({ respond: () => ({ ok: false, status: 500 }) });
  await openTab(on, 12, 'https://x.com/jack/status/20');
  assert.deepEqual(removed, []);
});

test('sends once when onCreated and onUpdated both fire for a tab', async () => {
  const { on, posted, removed } = loadWorker();
  on.created({ id: 13, pendingUrl: 'https://x.com/jack/status/20' });
  on.updated(13, { url: 'https://x.com/jack/status/20', status: 'loading' }, { id: 13 });
  on.updated(13, { status: 'complete' }, { id: 13, url: 'https://x.com/jack/status/20' });
  await settle();

  assert.equal(posted.length, 1);
  assert.deepEqual(removed, [13]);
});

test('follows a t.co redirect hop through to the tweet', async () => {
  const { on, posted, removed } = loadWorker();
  on.created({ id: 14, pendingUrl: 'https://t.co/shortened' });
  on.updated(14, { status: 'loading', url: 'https://t.co/shortened' }, { id: 14 });
  on.updated(14, { url: 'https://x.com/jack/status/20' }, { id: 14 });
  await settle();

  assert.equal(posted.length, 1);
  assert.deepEqual(removed, [14]);
});

test('stops watching a new tab once it settles on a non-tweet page', async () => {
  const { on, posted, removed } = loadWorker();
  on.created({ id: 15, pendingUrl: 'https://x.com/home' });
  on.updated(15, { status: 'complete', url: 'https://x.com/home' }, { id: 15 });
  await settle();

  // The user is now browsing in this tab; a later tweet must not close it.
  on.updated(15, { url: 'https://x.com/jack/status/20' }, { id: 15 });
  await settle();

  assert.deepEqual(posted, []);
  assert.deepEqual(removed, []);
});
