// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 abel0x <https://github.com/abel0x>

'use strict';

/**
 * X Link Collector -- MV3 service worker.
 *
 * A tab that opens an x.com/twitter.com status URL has its link posted to the
 * local receiver; once the receiver confirms the write, the tab is closed.
 *
 * Two rules keep this from ever eating a tab you cared about:
 *   1. Only tabs this worker saw being *created* are ever closed. A tweet you
 *      navigate to in an existing tab (clicking through the feed, which is a
 *      same-tab SPA navigation) is left completely alone.
 *   2. A tab is closed only after the receiver answers 2xx. If the receiver is
 *      down the tab stays open, so a link is never lost.
 */

/* -------------------------------------------------------------- settings */

const RECEIVER_URL = 'http://127.0.0.1:9876/';

/** Give up on a request after this long; a dead receiver must not stall captures. */
const REQUEST_TIMEOUT_MS = 2500;

/** After a failed send, stop trying for this long instead of retrying per tab. */
const COOLDOWN_MS = 15000;

/** How long a freshly created tab stays eligible for capture. */
const CANDIDATE_TTL_MS = 120000;

/** How long the toolbar badge shows the running count. */
const BADGE_CLEAR_MS = 4000;

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

/* ----------------------------------------------------------------- state */

/** tabId -> creation time. Only these tabs may be captured and closed. */
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
  const saved = await send(url);
  if (!saved) return; // Rule 2: unconfirmed means the tab stays open.

  candidates.delete(tabId);
  try {
    await chrome.tabs.remove(tabId);
  } catch {
    // Already closed by the user, or gone with its window. The link is saved.
  }
}

async function send(url) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);
  try {
    const response = await fetch(RECEIVER_URL, {
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
    setBadge(String(captureCount), '#1d9bf0', BADGE_CLEAR_MS);
    return true;
  } catch (error) {
    const reason = error.name === 'AbortError' ? 'timed out' : error.message;
    cooldownUntil = Date.now() + COOLDOWN_MS;
    cooldownLogged = false;
    console.warn(
      `[x-link-collector] could not save ${url}: ${reason}. Tab left open; ` +
        `pausing ${COOLDOWN_MS / 1000}s. Is x-link-receiver running?`
    );
    setBadge('!', '#d93025', 0);
    return false;
  } finally {
    clearTimeout(timer);
  }
}

/* ---------------------------------------------------------- tab tracking */

chrome.tabs.onCreated.addListener((tab) => {
  if (typeof tab.id !== 'number' || tab.id === chrome.tabs.TAB_ID_NONE) return;
  expireCandidates();
  candidates.set(tab.id, Date.now());
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
  for (const [tabId, created] of candidates) {
    if (created < cutoff) candidates.delete(tabId);
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

// Clicking the toolbar icon reports whether the receiver is up.
chrome.action.onClicked.addListener(async () => {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);
  try {
    const response = await fetch(`${RECEIVER_URL}health`, {
      cache: 'no-store',
      signal: controller.signal,
    });
    const info = await response.json();
    console.info('[x-link-collector] receiver up:', info);
    setBadge('ok', '#1d9bf0', BADGE_CLEAR_MS);
    cooldownUntil = 0;
    cooldownLogged = false;
  } catch (error) {
    console.warn('[x-link-collector] receiver down:', error.message);
    setBadge('!', '#d93025', 0);
  } finally {
    clearTimeout(timer);
  }
});

console.info(`[x-link-collector] worker ready; posting to ${RECEIVER_URL}`);
