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

/**
 * Load background.js into an isolated context and return its stubs.
 *
 * `tabs` are the tabs that already exist, by id, for chrome.tabs.get/query;
 * `stored` is what chrome.storage.local holds.
 */
function loadWorker({ respond = () => ({ ok: true, status: 200 }), tabs = {}, stored = {} } = {}) {
  const posted = [];
  const removed = [];
  const created = [];
  const focused = [];
  const opened = { options: 0 };
  const menus = [];
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
      posted.push({ url, body: init?.body, method: init?.method || 'GET', headers: init?.headers });
      const result = respond(posted.length, url);
      if (result instanceof Error) throw result;
      return { ...result, json: async () => ({ ok: true }) };
    },
    chrome: {
      storage: {
        local: { get: async (defaults) => ({ ...defaults, ...stored }) },
        onChanged: { addListener: (fn) => (on.storage = fn) },
      },
      tabs: {
        TAB_ID_NONE: -1,
        onCreated: { addListener: (fn) => (on.created = fn) },
        onUpdated: { addListener: (fn) => (on.updated = fn) },
        onRemoved: { addListener: (fn) => (on.removed = fn) },
        get: async (id) => {
          if (!tabs[id]) throw new Error(`No tab with id: ${id}`);
          return tabs[id];
        },
        query: async ({ url }) => Object.values(tabs).filter((t) => url.some((u) => t.url.startsWith(u.slice(0, -1)))),
        create: async (props) => {
          created.push(props.url);
          return { id: 99, ...props };
        },
        update: async (id, props) => {
          focused.push(id);
          return { id, ...props };
        },
        remove: async (id) => {
          removed.push(id);
        },
      },
      windows: { update: async () => ({}) },
      contextMenus: {
        removeAll: (done) => {
          menus.length = 0;
          done?.();
        },
        create: (props) => menus.push(props),
        onClicked: { addListener: (fn) => (on.menu = fn) },
      },
      i18n: { getMessage: (key) => key },
      runtime: {
        openOptionsPage: async () => {
          opened.options += 1;
        },
        onInstalled: { addListener: (fn) => (on.installed = fn) },
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
  const posts = () => posted.filter((p) => p.method === 'POST');
  return { sandbox, on, posted, posts, removed, created, focused, opened, menus };
}

/** Let the worker's promise chain run to completion. */
const settle = async () => {
  for (let i = 0; i < 8; i += 1) await new Promise((r) => setTimeout(r, 0));
};

const openTab = async (on, id, url, opener) => {
  on.created({ id, pendingUrl: url, openerTabId: opener });
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
  const { on, posts, removed } = loadWorker();
  await openTab(on, 7, 'https://x.com/jack/status/20?s=20');

  assert.equal(posts().length, 1);
  assert.equal(posts()[0].url, 'http://127.0.0.1:9876/');
  assert.equal(posts()[0].body, 'https://x.com/jack/status/20');
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
  const { on, posts, removed } = loadWorker({
    respond: () => new Error('ECONNREFUSED'),
  });
  await openTab(on, 8, 'https://x.com/jack/status/20');

  assert.equal(posts().length, 1, 'it tried once');
  assert.deepEqual(removed, [], 'tab must survive so the link is not lost');
});

test('backs off instead of retrying every tab while the receiver is down', async () => {
  const { on, posts } = loadWorker({ respond: () => new Error('ECONNREFUSED') });
  await openTab(on, 9, 'https://x.com/a/status/1');
  await openTab(on, 10, 'https://x.com/b/status/2');
  await openTab(on, 11, 'https://x.com/c/status/3');

  assert.equal(posts().length, 1, 'one failure, then cooldown');
});

test('does not close a tab on a non-2xx reply', async () => {
  const { on, removed } = loadWorker({ respond: () => ({ ok: false, status: 500 }) });
  await openTab(on, 12, 'https://x.com/jack/status/20');
  assert.deepEqual(removed, []);
});

test('sends once when onCreated and onUpdated both fire for a tab', async () => {
  const { on, posts, removed } = loadWorker();
  on.created({ id: 13, pendingUrl: 'https://x.com/jack/status/20' });
  on.updated(13, { url: 'https://x.com/jack/status/20', status: 'loading' }, { id: 13 });
  on.updated(13, { status: 'complete' }, { id: 13, url: 'https://x.com/jack/status/20' });
  await settle();

  assert.equal(posts().length, 1);
  assert.deepEqual(removed, [13]);
});

test('follows a t.co redirect hop through to the tweet', async () => {
  const { on, posts, removed } = loadWorker();
  on.created({ id: 14, pendingUrl: 'https://t.co/shortened' });
  on.updated(14, { status: 'loading', url: 'https://t.co/shortened' }, { id: 14 });
  on.updated(14, { url: 'https://x.com/jack/status/20' }, { id: 14 });
  await settle();

  assert.equal(posts().length, 1);
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

test('leaves a tweet opened from the panel alone', async () => {
  const { on, posted, removed } = loadWorker({
    tabs: { 1: { id: 1, windowId: 1, url: 'http://127.0.0.1:9876/#links' } },
  });
  await openTab(on, 16, 'https://x.com/jack/status/20', 1);
  // X rewrites its address as it loads; still the panel's tab.
  on.updated(16, { url: 'https://x.com/jack/status/20?s=1' }, { id: 16 });
  await settle();

  assert.deepEqual(posted, []);
  assert.deepEqual(removed, []);
});

test('still collects a tweet opened from any other page', async () => {
  const { on, posts, removed } = loadWorker({
    tabs: {
      1: { id: 1, windowId: 1, url: 'https://x.com/home' },
      2: { id: 2, windowId: 1, url: 'http://127.0.0.1:3000/' },
    },
  });
  await openTab(on, 17, 'https://x.com/jack/status/20', 1);
  await openTab(on, 18, 'https://x.com/jack/status/21', 2);
  // An opener that has since closed says nothing either way.
  await openTab(on, 19, 'https://x.com/jack/status/22', 404);

  assert.equal(posts().length, 3);
  assert.deepEqual(removed, [17, 18, 19]);
});

test('posts to the port set on the options page', async () => {
  const { on, posts, removed } = loadWorker({ stored: { port: 9988 } });
  await openTab(on, 20, 'https://x.com/jack/status/20');
  assert.equal(posts()[0].url, 'http://127.0.0.1:9988/');
  assert.deepEqual(removed, [20]);

  on.storage({ port: { newValue: 9999 } }, 'local');
  await openTab(on, 21, 'https://x.com/jack/status/21');
  assert.equal(posts()[1].url, 'http://127.0.0.1:9999/');
});

test('the toolbar icon opens the panel, or brings an open one forward', async () => {
  const first = loadWorker();
  await first.on.clicked();
  await settle();
  assert.equal(first.posted[0].url, 'http://127.0.0.1:9876/health');
  assert.equal(first.posted[0].headers['X-Link-Collector'], 'extension');
  assert.deepEqual(first.created, ['http://127.0.0.1:9876/']);

  const again = loadWorker({ tabs: { 5: { id: 5, windowId: 2, url: 'http://127.0.0.1:9876/#settings' } } });
  await again.on.clicked();
  await settle();
  assert.deepEqual(again.created, []);
  assert.deepEqual(again.focused, [5]);
});

test('the toolbar icon opens the options page when the receiver is down', async () => {
  const { on, created, opened } = loadWorker({ respond: () => new Error('ECONNREFUSED') });
  await on.clicked();
  await settle();
  assert.deepEqual(created, []);
  assert.equal(opened.options, 1);
});

test('the right-click menu collects a tweet without opening or closing a tab', async () => {
  const { on, posts, removed, created, menus } = loadWorker();
  on.installed();
  await settle();
  assert.deepEqual(menus.map((m) => m.id), ['collect-link', 'collect-page']);
  assert.ok(menus[0].targetUrlPatterns.includes('*://x.com/*/status/*'));

  on.menu({ menuItemId: 'collect-link', linkUrl: 'https://twitter.com/jack/status/20?s=20' });
  on.menu({ menuItemId: 'collect-page', pageUrl: 'https://x.com/jack/status/21/photo/1' });
  on.menu({ menuItemId: 'collect-link', linkUrl: 'https://x.com/jack' });
  await settle();

  assert.deepEqual(posts().map((p) => p.body), ['https://x.com/jack/status/20', 'https://x.com/jack/status/21']);
  assert.deepEqual(removed, []);
  assert.deepEqual(created, []);
});
