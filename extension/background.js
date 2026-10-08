// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 abel0x <https://github.com/abel0x>

'use strict';

/**
 * X Link Collector -- MV3 service worker.
 *
 * A tab that opens an x.com/twitter.com status URL has its link posted to the
 * local receiver; once the receiver confirms the write, the tab is closed.
 *
 * Three rules keep this from ever eating a tab you cared about:
 *   1. Only tabs this worker saw being *created* are ever closed. A tweet you
 *      navigate to in an existing tab (clicking through the feed, which is a
 *      same-tab SPA navigation) is left completely alone.
 *   2. A tab is closed only after the receiver answers 2xx. If the receiver is
 *      down the tab stays open, so a link is never lost.
 *   3. A tweet opened from the receiver's own panel is one you asked to read,
 *      not one to collect; its tab is left alone too.
 *
 * Clicking the toolbar icon opens the panel, or -- when the receiver does not
 * answer -- this extension's options page, which says how to start it. A right
 * click on a tweet link, or on a tweet's own page, collects it without opening
 * or closing anything.
 */

/* -------------------------------------------------------------- settings */

const DEFAULT_PORT = 9876;

/** Give up on a request after this long; a dead receiver must not stall captures. */
const REQUEST_TIMEOUT_MS = 2500;

/** After a failed send, stop trying for this long instead of retrying per tab. */
const COOLDOWN_MS = 15000;

/** How long a freshly created tab stays eligible for capture. */
const CANDIDATE_TTL_MS = 120000;

/** How long the toolbar badge shows the running count. */
const BADGE_CLEAR_MS = 4000;

const BLUE = '#1d9bf0';
const RED = '#d93025';

/** The receiver's port; the options page changes it when the receiver moves. */
let port = DEFAULT_PORT;

/** Settles once the stored port is known. Every request waits for it. */
const ready = (async () => {
  try {
    const stored = await chrome.storage.local.get({ port: DEFAULT_PORT });
    port = validPort(stored.port);
  } catch {
    // No stored settings to read; the default stands.
  }
})();

chrome.storage.onChanged.addListener((changes, area) => {
  if (area !== 'local' || !changes.port) return;
  port = validPort(changes.port.newValue);
  // A new address deserves a fresh try, not the old one's cooldown.
  cooldownUntil = 0;
  cooldownLogged = false;
});

function validPort(value) {
  const n = Number(value);
  return Number.isInteger(n) && n >= 1 && n <= 65535 ? n : DEFAULT_PORT;
}

const receiverUrl = (path = '') => `http://127.0.0.1:${port}/${path}`;

/* -------------------------------------------------------------- matching */

const TWEET_HOSTS = new Set(['x.com', 'twitter.com']);
const HOST_PREFIXES = ['www.', 'mobile.', 'm.'];

// /alice/status/12345, /alice/statuses/12345, plus any trailing /photo/1 etc.
const USER_STATUS = /^\/([A-Za-z0-9_]{1,15})\/status(?:es)?\/(\d{1,25})(?:\/|$)/;
// /i/status/12345 and /i/web/status/12345 -- author unknown.
const I_STATUS = /^\/i\/(?:web\/)?status\/(\d{1,25})(?:\/|$)/;

/**
 * Reduce any status URL to one canonical form, dropping query strings,
 * tracking parameters (?s=20&t=...), fragments and trailing path junk.
 * Returns null for anything that is not a tweet permalink.
 */
function canonicalTweetUrl(raw) {
  if (!raw) return null;

  let parsed;
  try {
    parsed = new URL(raw);
  } catch {
    return null; // about:blank, chrome://newtab, garbage
  }
  if (parsed.protocol !== 'https:' && parsed.protocol !== 'http:') return null;

  let host = parsed.hostname.toLowerCase();
  for (const prefix of HOST_PREFIXES) {
    if (host.startsWith(prefix)) {
      host = host.slice(prefix.length);
      break;
    }
  }
  if (!TWEET_HOSTS.has(host)) return null;

  const anonymous = I_STATUS.exec(parsed.pathname);
  if (anonymous) return `https://x.com/i/web/status/${anonymous[1]}`;

  const named = USER_STATUS.exec(parsed.pathname);
  if (named) return `https://x.com/${named[1]}/status/${named[2]}`;

  return null;
}

/** The receiver's panel, under any of the names loopback goes by. */
function isPanel(raw) {
  let parsed;
  try {
    parsed = new URL(raw);
  } catch {
    return false;
  }
  return parsed.protocol === 'http:'
    && Number(parsed.port || 80) === port
    && ['127.0.0.1', 'localhost', '[::1]'].includes(parsed.hostname);
}

/* ----------------------------------------------------------------- state */

/** tabId -> { created, opener }. Only these tabs may be captured and closed. */
const candidates = new Map();
/** tabIds with a request in flight, so one tab is never sent twice at once. */
const inFlight = new Set();

let cooldownUntil = 0;
let cooldownLogged = false;
let captureCount = 0;

// This is all deliberately in-memory. MV3 stops the worker when idle and the
// state is rebuilt from the next tab event; the receiver owns deduplication,
// so losing it costs nothing.

/* --------------------------------------------------------------- capture */

function consider(tabId, rawUrl) {
  const url = canonicalTweetUrl(rawUrl);
  if (!url) return;

  // Rule 1: a tab we did not open is the user's tab. Never touch it.
  if (!candidates.has(tabId)) return;
  if (inFlight.has(tabId)) return;

  if (Date.now() < cooldownUntil) {
    if (!cooldownLogged) {
      cooldownLogged = true;
      console.info(
        `[x-link-collector] receiver still unreachable; leaving tabs open until ` +
          `${new Date(cooldownUntil).toLocaleTimeString()}`
      );
    }
    return;
  }

  inFlight.add(tabId);
  capture(tabId, url).finally(() => inFlight.delete(tabId));
}

async function capture(tabId, url) {
  // Rule 3: a tweet opened from the panel is there to be read.
  if (await openedByPanel(candidates.get(tabId))) {
    candidates.delete(tabId);
    return;
  }

  const saved = await send(url);
  if (!saved) return; // Rule 2: unconfirmed means the tab stays open.

  candidates.delete(tabId);
  try {
    await chrome.tabs.remove(tabId);
  } catch {
    // Already closed by the user, or gone with its window. The link is saved.
  }
}

async function openedByPanel(candidate) {
  if (!candidate || typeof candidate.opener !== 'number') return false;
  try {
    const opener = await chrome.tabs.get(candidate.opener);
    await ready;
    return isPanel(opener.url || opener.pendingUrl);
  } catch {
    return false; // The opener is already gone; nothing says it was the panel.
  }
}

async function send(url) {
  await ready;
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);
  try {
    const response = await fetch(receiverUrl(), {
      method: 'POST',
      // text/plain is a CORS-safelisted content type, so this stays a "simple
      // request" and the browser never pays for a preflight on the hot path.
      headers: { 'Content-Type': 'text/plain;charset=UTF-8' },
      body: url,
      cache: 'no-store',
      signal: controller.signal,
    });
    if (!response.ok) throw new Error(`receiver replied ${response.status}`);

    cooldownUntil = 0;
    cooldownLogged = false;
    captureCount += 1;
    setBadge(String(captureCount), BLUE, BADGE_CLEAR_MS);
    return true;
  } catch (error) {
    const reason = error.name === 'AbortError' ? 'timed out' : error.message;
    cooldownUntil = Date.now() + COOLDOWN_MS;
    cooldownLogged = false;
    console.warn(
      `[x-link-collector] could not save ${url}: ${reason}. Tab left open; ` +
        `pausing ${COOLDOWN_MS / 1000}s. Is x-link-receiver running on port ${port}?`
    );
    setBadge('!', RED, 0);
    return false;
  } finally {
    clearTimeout(timer);
  }
}

/** Whether the receiver answers. The header tells it an extension is installed. */
async function healthy() {
  await ready;
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);
  try {
    const response = await fetch(receiverUrl('health'), {
      headers: { 'X-Link-Collector': 'extension' },
      cache: 'no-store',
      signal: controller.signal,
    });
    return response.ok;
  } catch {
    return false;
  } finally {
    clearTimeout(timer);
  }
}

/* ---------------------------------------------------------- tab tracking */

chrome.tabs.onCreated.addListener((tab) => {
  if (typeof tab.id !== 'number' || tab.id === chrome.tabs.TAB_ID_NONE) return;
  expireCandidates();
  candidates.set(tab.id, { created: Date.now(), opener: tab.openerTabId });
  // Middle-clicking a link gives us the destination in pendingUrl right away.
  consider(tab.id, tab.pendingUrl || tab.url);
});

function onTabUpdated(tabId, changeInfo, tab) {
  const url = changeInfo.url || tab.pendingUrl || tab.url;
  if (!url) return;

  if (canonicalTweetUrl(url)) {
    consider(tabId, url);
    return;
  }

  // The tab settled on something that is not a tweet, so it has become a tab
  // the user is browsing in. Stop watching it. Redirect hops (t.co) never
  // reach 'complete', so they survive this check.
  if (changeInfo.status === 'complete' && /^https?:/i.test(url)) {
    candidates.delete(tabId);
  }
}

// The properties filter cuts out title/favicon churn; not every Chromium build
// supports it, so fall back to the unfiltered listener.
try {
  chrome.tabs.onUpdated.addListener(onTabUpdated, { properties: ['status', 'url'] });
} catch {
  chrome.tabs.onUpdated.addListener(onTabUpdated);
}

chrome.tabs.onRemoved.addListener((tabId) => {
  candidates.delete(tabId);
  inFlight.delete(tabId);
});

function expireCandidates() {
  const cutoff = Date.now() - CANDIDATE_TTL_MS;
  for (const [tabId, candidate] of candidates) {
    if (candidate.created < cutoff) candidates.delete(tabId);
  }
}

/* ------------------------------------------------------------------- ui */

let badgeTimer = null;

function setBadge(text, color, clearAfterMs) {
  const ignore = () => {};
  chrome.action.setBadgeBackgroundColor({ color }).catch(ignore);
  chrome.action.setBadgeText({ text }).catch(ignore);
  if (badgeTimer) clearTimeout(badgeTimer);
  if (clearAfterMs > 0) {
    badgeTimer = setTimeout(() => chrome.action.setBadgeText({ text: '' }).catch(ignore), clearAfterMs);
  }
}

/** Bring a panel tab that is already open to the front, or open one. */
async function showPanel() {
  const [open] = await chrome.tabs.query({
    url: [`http://127.0.0.1:${port}/*`, `http://localhost:${port}/*`],
  });
  if (!open) {
    await chrome.tabs.create({ url: receiverUrl() });
    return;
  }
  await chrome.tabs.update(open.id, { active: true });
  await chrome.windows.update(open.windowId, { focused: true });
}

chrome.action.onClicked.addListener(async () => {
  if (await healthy()) {
    cooldownUntil = 0;
    cooldownLogged = false;
    setBadge('', BLUE, 0);
    await showPanel().catch((error) => console.warn('[x-link-collector] cannot open the panel:', error.message));
  } else {
    setBadge('!', RED, 0);
    await chrome.runtime.openOptionsPage();
  }
});

/* --------------------------------------------------------- context menu */

const TWEET_PATTERNS = [
  '*://x.com/*/status/*',
  '*://*.x.com/*/status/*',
  '*://twitter.com/*/status/*',
  '*://*.twitter.com/*/status/*',
];

function createMenus() {
  // Recreated from scratch, so an update never trips over an existing id.
  chrome.contextMenus.removeAll(() => {
    chrome.contextMenus.create({
      id: 'collect-link',
      title: chrome.i18n.getMessage('menuLink'),
      contexts: ['link'],
      targetUrlPatterns: TWEET_PATTERNS,
    });
    chrome.contextMenus.create({
      id: 'collect-page',
      title: chrome.i18n.getMessage('menuPage'),
      contexts: ['page'],
      documentUrlPatterns: TWEET_PATTERNS,
    });
  });
}

// Nothing is closed from here: the link was never opened, and the page is
// the one being read.
chrome.contextMenus.onClicked.addListener((info) => {
  const url = canonicalTweetUrl(info.menuItemId === 'collect-link' ? info.linkUrl : info.pageUrl);
  if (url) send(url);
});

// Menus are set up once per install or update. The hello lets a running panel
// stop showing how to load the extension.
chrome.runtime.onInstalled.addListener(() => {
  createMenus();
  healthy();
});

ready.then(() => console.info(`[x-link-collector] worker ready; posting to ${receiverUrl()}`));
