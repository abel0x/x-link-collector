// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 abel0x <https://github.com/abel0x>
'use strict';

/*
 * x-link-collector panel. Plain script, no build step: the receiver serves this
 * file as is from inside its own binary, and it talks to nothing but the
 * receiver's /api/ on the same origin.
 */

/* ----------------------------------------------------------------- words */

/* WORDS comes from /words.js: i18n/en.json and i18n/tr.json, served together.
   English is the source; Turkish carries every key English has. */

/* x-download records why a link failed in English, for its own log and the
   command line. The ones it writes itself are put into the reader's language
   here; anything else is a tool's own output and is shown as it came. */
const WHY = {
  'tweet not found (deleted, private account, or bad id)': 'why.notFound',
  'restricted tweet -- retry with --mirror, or --cookies vivaldi': 'why.restricted',
  'tweet is restricted (age-gated or hidden from logged-out viewers)': 'why.restrictedAge',
  'login required -- retry with --cookies vivaldi': 'why.login',
  'rate limited by X -- wait a bit, or lower --jobs': 'why.rateLimited',
  'account suspended': 'why.suspended',
  'network error': 'why.network',
  'not a tweet URL': 'why.notTweet',
  'not an X link; only tweets on x.com are downloaded': 'why.notX',
  'no mirror had this tweet': 'why.noMirror',
};

/* -------------------------------------------------------------- plumbing */

const $ = (selector) => document.querySelector(selector);

/** Browser storage holds preferences only, and may be missing or blocked. */
function remember(key, value) {
  try {
    if (value === undefined) return localStorage.getItem(`x-link-collector.${key}`);
    localStorage.setItem(`x-link-collector.${key}`, value);
  } catch {
    // fine without it
  }
  return null;
}

let lang = remember('lang') || ((navigator.language || '').toLowerCase().startsWith('tr') ? 'tr' : 'en');
if (!WORDS[lang]) lang = 'en';
let fmt = null;

function makeFormatters() {
  const locale = lang === 'tr' ? 'tr-TR' : 'en-GB';
  fmt = {
    number: new Intl.NumberFormat(locale),
    decimal: new Intl.NumberFormat(locale, { maximumFractionDigits: 1 }),
    date: new Intl.DateTimeFormat(locale, { day: 'numeric', month: 'short', year: 'numeric' }),
    day: new Intl.DateTimeFormat(locale, { weekday: 'short', day: 'numeric', month: 'short' }),
    relative: new Intl.RelativeTimeFormat(locale, { numeric: 'auto' }),
  };
}

/** A string from WORDS; `{name}` is filled from `vars`, `vars.n` picks a plural. */
function t(key, vars = {}) {
  let text = WORDS[lang][key] ?? WORDS.en[key] ?? key;
  if (Array.isArray(text)) text = vars.n === 1 ? text[0] : text[1];
  return text.replace(/\{(\w+)\}/g, (whole, name) => {
    if (!(name in vars)) return whole;
    const value = vars[name];
    return typeof value === 'number' ? fmt.number.format(value) : String(value);
  });
}

/** Why a link failed, in the reader's language when x-download said it. */
function whyFailed(error) {
  const key = WHY[error];
  if (key) return h('p', { class: 'why', text: t(key) });
  return h('p', { class: 'why', lang: 'en', text: error });
}

function h(tag, props = {}, ...children) {
  const node = document.createElement(tag);
  for (const [key, value] of Object.entries(props)) {
    if (value === undefined || value === null || value === false) continue;
    if (key === 'class') node.className = value;
    else if (key === 'text') node.textContent = value;
    else if (key.startsWith('on')) node.addEventListener(key.slice(2), value);
    else node.setAttribute(key, value === true ? '' : value);
  }
  for (const child of children.flat()) {
    if (child === null || child === undefined || child === false) continue;
    node.append(child instanceof Node ? child : String(child));
  }
  return node;
}

function icon(name) {
  const ns = 'http://www.w3.org/2000/svg';
  const svg = document.createElementNS(ns, 'svg');
  svg.setAttribute('class', 'icon');
  svg.setAttribute('aria-hidden', 'true');
  const use = document.createElementNS(ns, 'use');
  use.setAttribute('href', `#i-${name}`);
  svg.append(use);
  return svg;
}

/**
 * A refusal from the receiver carries a code and the values its sentence
 * needs; `explain` puts it into the reader's language. The English `error`
 * text stands in for anything without a code.
 */
/** The receiver writes this into the page; every change sent carries it. */
const PANEL_KEY = document.querySelector('meta[name="panel-key"]')?.content || '';

async function request(path, init = {}) {
  let res;
  try {
    res = await fetch(path, { cache: 'no-store', ...init, headers: { 'X-Panel-Key': PANEL_KEY, ...init.headers } });
  } catch {
    throw Object.assign(new Error(WORDS.en['err.network']), { code: 'network' });
  }
  const data = await res.json().catch(() => ({}));
  // A key this page does not have: it was served before the key changed.
  if (res.status === 403 && data.code === 'panel_key') location.reload();
  if (!res.ok) {
    throw Object.assign(new Error(data.error || `HTTP ${res.status}`), { code: data.code, vars: data.vars });
  }
  return data;
}

const getJSON = (path) => request(path);

const postJSON = (path, body) => request(path, {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify(body ?? {}),
});

function explain(error) {
  const key = `err.${error.code}`;
  if (!error.code || !(key in WORDS.en)) return error.message;
  const vars = { ...(error.vars || {}) };
  if ('reason' in vars) {
    const reason = `reason.${vars.reason}`;
    vars.reason = vars.reason && reason in WORDS.en ? t(reason) : vars.detail || '';
  }
  return t(key, vars);
}

function toast(message, kind = '') {
  const node = h('div', { class: `toast ${kind}`, role: kind === 'error' ? 'alert' : 'status', text: message });
  $('#toasts').append(node);
  setTimeout(() => node.remove(), kind === 'error' ? 8000 : 3500);
}

function confirmAction({ title, body, action }) {
  const dialog = $('#confirm');
  $('#confirm-title').textContent = title;
  $('#confirm-body').textContent = body;
  $('#confirm-ok').textContent = action;
  $('#confirm-cancel').textContent = t('cancel');
  return new Promise((resolve) => {
    const done = (answer) => {
      dialog.close();
      resolve(answer);
    };
    $('#confirm-ok').onclick = () => done(true);
    $('#confirm-cancel').onclick = () => done(false);
    dialog.oncancel = () => resolve(false);
    dialog.showModal();
  });
}

async function copyText(text) {
  try {
    await navigator.clipboard.writeText(text);
    toast(t('copied'));
  } catch (e) {
    toast(explain(e), 'error');
  }
}

/* ------------------------------------------------------------ formatting */

const num = (n) => fmt.number.format(n);

/** The receiver's clock, so "2 minutes ago" is right even if this one is off. */
function serverNow() {
  const s = state.status;
  return s ? s.now + (Date.now() - state.fetchedAt) / 1000 : Date.now() / 1000;
}

function ago(unix) {
  const diff = unix - serverNow();
  const abs = Math.abs(diff);
  if (abs < 45) return fmt.relative.format(0, 'second');
  const steps = [[3600, 60, 'minute'], [86400, 3600, 'hour'], [2592000, 86400, 'day'], [31536000, 2592000, 'month']];
  for (const [limit, size, unit] of steps) {
    if (abs < limit) return fmt.relative.format(Math.round(diff / size), unit);
  }
  return fmt.relative.format(Math.round(diff / 31536000), 'year');
}

function duration(seconds) {
  const tr = lang === 'tr';
  if (seconds < 60) return `${Math.max(1, Math.round(seconds))} ${tr ? 'sn' : 's'}`;
  if (seconds < 3600) return `${Math.round(seconds / 60)} ${tr ? 'dk' : 'min'}`;
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.round((seconds % 3600) / 60);
  return tr ? `${hours} sa ${minutes} dk` : `${hours} h ${minutes} min`;
}

function bytes(n) {
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let i = 0;
  while (n >= 1024 && i < units.length - 1) {
    n /= 1024;
    i += 1;
  }
  return `${i ? fmt.decimal.format(n) : fmt.number.format(n)} ${units[i]}`;
}

/** Tweet ids since late 2010 carry their creation time in the top bits. */
function postedAt(id) {
  try {
    const big = BigInt(id);
    if (big < 29700859247n) return null;
    return new Date(Number((big >> 22n) + 1288834974657n));
  } catch {
    return null;
  }
}

/** The account for a tweet; for any other link, the site it is on. */
function who(item) {
  if (item.handle) return `@${item.handle}`;
  if (item.id) return t('links.unknownAuthor');
  try {
    return new URL(item.url).hostname.replace(/^www\./, '');
  } catch {
    return item.url;
  }
}

function stateChip(status) {
  return h('span', { class: `state ${status}` }, icon(status), t(`state.${status}`));
}

/* ----------------------------------------------------------------- state */

const VIEWS = ['overview', 'links', 'gallery', 'download', 'folder', 'settings'];
const ACTIVE = ['running', 'stopping'];

const state = {
  view: 'overview',
  status: null,
  fetchedAt: 0,
  offline: false,
  job: null,
  jobId: 0,
  lines: [],
  shown: 0,
  cursor: 0,
  settings: null,
  draft: {},
  tools: null,
  toolsBusy: false,
  list: { q: '', filter: 'all', items: [], matched: 0, total: 0, busy: false, sig: '' },
  latestSig: '',
  setupSig: '',
  barsSig: '',
  dlLang: '',
  dlCookies: null,
  suggestUpdate: false,
  timer: 0,
  ticking: false,
};

const isActive = (job) => Boolean(job && ACTIVE.includes(job.state));

/* ------------------------------------------------------------------ loop */

async function tick() {
  if (state.ticking) return;
  state.ticking = true;
  clearTimeout(state.timer);
  try {
    await refreshStatus();
    if (!state.offline && (isActive(state.status?.job) || ['download', 'folder', 'settings'].includes(state.view))) {
      await refreshJob();
    }
  } finally {
    state.ticking = false;
    const busy = isActive(state.status?.job);
    state.timer = setTimeout(tick, document.hidden ? 15000 : busy ? 1000 : 2500);
  }
}

/** Refresh now rather than at the next tick, e.g. right after an action. */
function kick() {
  setTimeout(tick, 0);
}

async function refreshStatus() {
  let s;
  try {
    s = await getJSON(`/api/status?strip=1&tz=${new Date().getTimezoneOffset()}`);
  } catch {
    setOffline(true);
    return;
  }
  const before = state.status;
  state.status = s;
  state.fetchedAt = Date.now();
  setOffline(false);
  renderChrome();
  if (before && isActive(before.job) && !isActive(s.job)) jobEnded(s.job, before, s);
  if (state.view === 'overview') renderOverview(before);
  if (state.view === 'links') renderLinksMeta(before);
  if (state.view === 'download') renderDownload();
  if (state.view === 'folder') renderFolder();
  if (state.view === 'settings') renderAbout();
}

function setOffline(offline) {
  if (state.offline === offline) return;
  state.offline = offline;
  $('#offline').hidden = !offline;
  renderChrome();
}

async function refreshJob() {
  let data;
  try {
    data = await getJSON(`/api/job?since=${state.cursor}`);
  } catch {
    return;
  }
  if (data.job && data.job.id !== state.jobId) {
    state.jobId = data.job.id;
    state.lines = [];
    state.shown = 0;
    $('#job-log').replaceChildren();
  }
  state.job = data.job;
  if (data.lines.length) {
    state.lines.push(...data.lines);
    if (state.lines.length > 4000) {
      state.lines.splice(0, state.lines.length - 4000);
      state.shown = 0;
      $('#job-log').replaceChildren();
    }
  }
  state.cursor = Math.max(state.cursor, data.next);
  renderJob();
}

function jobEnded(job, before, after) {
  if (!job) return;
  if (job.kind === 'download' && !job.summary.startsWith('dry run') && !job.summary.startsWith('nothing to')) {
    downloadEnded(job, before.downloads, after.downloads);
  }
  if (job.kind === 'setup') {
    state.suggestUpdate = false;
    loadTools(true);
  }
  if (state.view === 'links') loadLinks(true);
  if (state.view === 'gallery') loadGallery(true);
  state.latestSig = '';
  kick();
}

/** What a finished download did, said in the panel and, if allowed, by the system. */
function downloadEnded(job, before, after) {
  const got = Math.max(0, after.done - before.done);
  const failed = Math.max(0, after.failed - before.failed);
  const said = [];
  if (got) said.push(t('notify.got', { n: got }));
  if (failed) said.push(t('notify.failed', { n: failed }));
  const title = t(job.state === 'stopped' ? 'notify.stopped' : 'notify.done');
  const body = said.length ? said.join(', ') : t('notify.nothing');
  toast(`${title}: ${body}`);
  // Nothing came down and several links broke: X has likely changed again.
  if (!got && failed >= 3) {
    state.suggestUpdate = true;
    state.setupSig = '';
  }
  if (document.hidden && 'Notification' in window && Notification.permission === 'granted') {
    try {
      new Notification(title, { body, icon: '/icon.png', tag: 'x-link-collector' });
    } catch {
      // some browsers only allow notifications from a service worker
    }
  }
}

/** Asked once, from the click that starts a download, which is when it makes sense. */
function askToNotify() {
  if ('Notification' in window && Notification.permission === 'default') {
    Notification.requestPermission().catch(() => {});
  }
}

async function startJob(body, message) {
  if (body.kind === 'download' && !body.dry_run) askToNotify();
  try {
    await postJSON('/api/job', body);
    if (message) toast(message);
    await tick();
  } catch (e) {
    toast(explain(e), 'error');
  }
}

/* ---------------------------------------------------------------- chrome */

function applyStatic() {
  document.documentElement.lang = lang;
  for (const node of document.querySelectorAll('[data-t]')) node.textContent = t(node.dataset.t);
  for (const node of document.querySelectorAll('[data-t-placeholder]')) node.placeholder = t(node.dataset.tPlaceholder);
  for (const node of document.querySelectorAll('[data-t-label]')) node.setAttribute('aria-label', t(node.dataset.tLabel));
  $('.tabs').setAttribute('aria-label', t('nav.label'));
  const langButton = $('#lang');
  langButton.textContent = t('lang.name');
  langButton.setAttribute('aria-label', t('lang.label'));
  langButton.lang = lang === 'tr' ? 'en' : 'tr';
  renderThemeButton();
}

function renderChrome() {
  const s = state.status;
  const conn = $('#conn');
  conn.classList.toggle('lost', state.offline);
  $('#conn-text').textContent = state.offline || !s ? t('conn.lost') : t('conn.ok', { port: String(s.port) });
  const restart = $('#restart');
  const want = state.settings?.settings?.port;
  restart.hidden = !(s && s.restart_needed && want);
  if (!restart.hidden) restart.textContent = t('restart', { want: String(want), port: String(s.port) });
  document.title = `${t(`nav.${state.view}`)} – x-link-collector`;
}

function currentTheme() {
  return document.documentElement.dataset.theme
    || (matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light');
}

function renderThemeButton() {
  const dark = currentTheme() === 'dark';
  const button = $('#theme');
  button.setAttribute('aria-label', t(dark ? 'theme.toLight' : 'theme.toDark'));
  button.title = button.getAttribute('aria-label');
  button.querySelector('use').setAttribute('href', dark ? '#i-sun' : '#i-moon');
}

function switchTheme() {
  const next = currentTheme() === 'dark' ? 'light' : 'dark';
  document.documentElement.dataset.theme = next;
  remember('theme', next);
  renderThemeButton();
  drawStrip();
}

function switchLanguage() {
  lang = lang === 'tr' ? 'en' : 'tr';
  remember('lang', lang);
  makeFormatters();
  applyStatic();
  state.latestSig = '';
  state.setupSig = '';
  state.barsSig = '';
  renderChrome();
  renderView();
}

/* ---------------------------------------------------------------- router */

function route() {
  const wanted = location.hash.slice(1);
  const view = VIEWS.includes(wanted) ? wanted : 'overview';
  const moved = state.view !== view;
  state.view = view;
  for (const name of VIEWS) $(`#view-${name}`).hidden = name !== view;
  for (const link of document.querySelectorAll('.tabs a')) {
    if (link.dataset.view === view) link.setAttribute('aria-current', 'page');
    else link.removeAttribute('aria-current');
  }
  const slot = document.querySelector(`#view-${view} .job-slot`);
  if (slot) slot.append($('#job'));
  renderChrome();
  renderView();
  if (moved) {
    window.scrollTo(0, 0);
    document.querySelector(`#view-${view} h1`)?.focus({ preventScroll: true });
  }
  kick();
}

function renderView() {
  switch (state.view) {
    case 'overview':
      renderOverview(null);
      break;
    case 'links':
      renderFilters();
      loadLinks(true);
      break;
    case 'gallery':
      loadGallery(true);
      break;
    case 'download':
      renderDownload();
      renderJob();
      break;
    case 'folder':
      renderFolder(true);
      renderJob();
      break;
    case 'settings':
      renderSettings();
      renderTools();
      renderAbout();
      renderJob();
      break;
    default:
      break;
  }
}

/* -------------------------------------------------------------- overview */

function renderOverview(before) {
  const s = state.status;
  if (!s) return;
  const d = s.downloads;
  const job = s.job;
  const downloading = isActive(job) && job.kind === 'download';

  $('#ov-title').textContent = s.total ? t('ov.title', { n: s.total }) : t('ov.titleEmpty');
  let sub;
  if (!s.total) sub = t('ov.subEmpty');
  else if (downloading) sub = t('ov.subRunning');
  else if (d.queued > d.pending && d.pending) sub = t('ov.subRetrying', { n: d.pending, k: d.queued - d.pending });
  else if (d.queued > d.pending) sub = t('ov.subRetryOnly', { n: d.queued });
  else if (d.pending) sub = t('ov.subWaiting', { n: d.pending });
  else if (d.failed && d.no_media) sub = t('ov.subUnfinished', { failed: d.failed, nomedia: d.no_media });
  else if (d.failed) sub = t('ov.subFailed', { n: d.failed });
  else sub = t('ov.subDone');
  if (s.auto_download && s.total && !downloading) sub = `${sub} ${t('ov.subAuto')}`;
  $('#ov-sub').textContent = sub;

  renderGoButton($('#ov-go'), $('#ov-stop'));
  $('#strip-figure').hidden = !s.total;
  if (!before || before.strip !== s.strip || before.total !== s.total) drawStrip();
  renderLegend();
  renderSetup();
  renderBars();
  renderLatest();
}

/** The one action that matters most right now, on the overview and the download page. */
function renderGoButton(go, stop) {
  const s = state.status;
  if (!s) return;
  const d = s.downloads;
  const job = s.job;
  if (stop) stop.hidden = !(isActive(job) && job.kind === 'download');
  go.dataset.retry = '';
  if (isActive(job)) {
    go.disabled = true;
    if (job.kind !== 'download') go.textContent = t('go.busy');
    else if (job.total) go.textContent = t('go.running', { done: job.done, total: job.total });
    else go.textContent = t('go.starting');
  } else if (d.queued) {
    // A plain run: the new links, and failed ones not yet given up on.
    go.disabled = false;
    go.textContent = t('go.download', { n: d.queued });
  } else if (d.failed) {
    // --retry-failed takes the media-less links along, so they are counted.
    go.disabled = false;
    go.dataset.retry = '1';
    go.textContent = t('go.retry', { n: d.failed + d.no_media });
  } else {
    go.disabled = true;
    go.textContent = t('go.nothing');
  }
}

function downloadBody(extra = {}) {
  const body = { kind: 'download', ...extra };
  if (state.view === 'download') {
    if ($('#dl-retry').checked) body.retry_failed = true;
    const limit = parseInt($('#dl-limit').value, 10);
    if (limit > 0) body.limit = limit;
  }
  return body;
}

/* ----------------------------------------------------------------- strip */

/*
 * One square per link, in links.txt order, coloured by what x-download made
 * of it. Drawn on a canvas: a collection of a few thousand links would be a
 * few thousand DOM nodes otherwise. Past what fits, squares stand for groups.
 */
const strip = { cell: 10, gap: 2, cols: 1, cells: 0, group: 1, focus: -1, info: new Map(), pending: 0 };
const STRIP_HEIGHT = 168;
const CODE = { d: 'done', n: 'no-media', f: 'failed', p: 'pending' };

function stripCode(i) {
  const codes = state.status?.strip || '';
  if (strip.group === 1) return codes[i] || 'p';
  const counts = {};
  const end = Math.min(codes.length, (i + 1) * strip.group);
  for (let j = i * strip.group; j < end; j += 1) counts[codes[j]] = (counts[codes[j]] || 0) + 1;
  return Object.keys(counts).sort((a, b) => counts[b] - counts[a])[0] || 'p';
}

function drawStrip() {
  const canvas = $('#strip');
  const codes = state.status?.strip || '';
  const width = $('#strip-area').clientWidth;
  if (!width || state.view !== 'overview') return;

  let group = 1;
  for (;;) {
    let fitted = false;
    for (const cell of [14, 12, 10, 8, 6, 5, 4, 3]) {
      const gap = cell >= 6 ? 2 : 1;
      const cols = Math.max(1, Math.floor((width + gap) / (cell + gap)));
      const cells = Math.ceil(codes.length / group);
      const rows = Math.ceil(cells / cols);
      if (rows * (cell + gap) - gap <= STRIP_HEIGHT) {
        Object.assign(strip, { cell, gap, cols, cells, group, rows });
        fitted = true;
        break;
      }
    }
    if (fitted || group > codes.length) break;
    group *= 2;
  }

  const height = Math.max(0, strip.rows * (strip.cell + strip.gap) - strip.gap);
  const ratio = window.devicePixelRatio || 1;
  canvas.style.height = `${height}px`;
  canvas.width = Math.round(width * ratio);
  canvas.height = Math.round(height * ratio);
  const ctx = canvas.getContext('2d');
  ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
  ctx.clearRect(0, 0, width, height);

  const css = getComputedStyle(document.documentElement);
  const colors = {};
  for (const [code, name] of Object.entries(CODE)) colors[code] = css.getPropertyValue(`--${name}`).trim();
  const radius = strip.cell >= 6 ? 2 : 1;
  for (let i = 0; i < strip.cells; i += 1) {
    const x = (i % strip.cols) * (strip.cell + strip.gap);
    const y = Math.floor(i / strip.cols) * (strip.cell + strip.gap);
    ctx.fillStyle = colors[stripCode(i)];
    ctx.beginPath();
    if (ctx.roundRect) ctx.roundRect(x, y, strip.cell, strip.cell, radius);
    else ctx.rect(x, y, strip.cell, strip.cell);
    ctx.fill();
  }
  if (strip.focus >= 0 && strip.focus < strip.cells) {
    const x = (strip.focus % strip.cols) * (strip.cell + strip.gap);
    const y = Math.floor(strip.focus / strip.cols) * (strip.cell + strip.gap);
    ctx.strokeStyle = css.getPropertyValue('--ink').trim();
    ctx.lineWidth = 2;
    ctx.strokeRect(x - 1.5, y - 1.5, strip.cell + 3, strip.cell + 3);
  }

  const d = state.status?.downloads;
  if (d) {
    canvas.setAttribute('aria-label', t('strip.label', {
      total: state.status.total, done: d.done, nomedia: d.no_media, failed: d.failed, waiting: d.pending,
    }));
  }
}

function renderLegend() {
  const d = state.status.downloads;
  const items = [['done', d.done], ['no-media', d.no_media], ['failed', d.failed], ['pending', d.pending]];
  $('#legend').replaceChildren(...items.map(([key, n]) => h('li', {},
    h('span', { class: `swatch ${key}`, 'aria-hidden': 'true' }),
    h('span', { text: t(`state.${key}`) }),
    h('strong', { text: num(n) }))));
  $('#legend-note').textContent = strip.group > 1 ? t('strip.noteGrouped', { k: strip.group }) : t('strip.note');
}

function cellAt(event) {
  const rect = $('#strip').getBoundingClientRect();
  const step = strip.cell + strip.gap;
  const col = Math.floor((event.clientX - rect.left) / step);
  const row = Math.floor((event.clientY - rect.top) / step);
  if (col < 0 || col >= strip.cols || row < 0) return -1;
  const i = row * strip.cols + col;
  return i < strip.cells ? i : -1;
}

function showStripTip(i) {
  const tip = $('#strip-tip');
  if (i < 0) {
    tip.hidden = true;
    return;
  }
  const total = state.status.total;
  const from = i * strip.group + 1;
  const to = Math.min(total, (i + 1) * strip.group);
  const step = strip.cell + strip.gap;
  const lines = [
    h('strong', { text: t(`state.${CODE[stripCode(i)]}`) }),
    h('span', { text: from === to ? t('strip.line', { n: from, total }) : t('strip.lines', { from, to, total }) }),
  ];
  const info = strip.info.get(from);
  if (strip.group === 1 && info) lines.push(h('span', { text: who(info) }));
  tip.replaceChildren(...lines);
  tip.hidden = false;
  const area = $('#strip-area').clientWidth;
  const center = (i % strip.cols) * step + strip.cell / 2;
  const half = tip.offsetWidth / 2;
  tip.style.left = `${Math.min(Math.max(center, half), area - half)}px`;
  tip.style.top = `${Math.floor(i / strip.cols) * step}px`;
  if (strip.group === 1 && !info) fetchLineInfo(from, i);
}

/** Who a square is: one link, fetched when the pointer rests on it. */
function fetchLineInfo(line, cell) {
  clearTimeout(strip.pending);
  strip.pending = setTimeout(async () => {
    try {
      const offset = state.status.total - line;
      const data = await getJSON(`/api/links?offset=${offset}&limit=1`);
      const item = data.items[0];
      if (item && item.line === line) {
        strip.info.set(line, item);
        if (!$('#strip-tip').hidden && strip.hover === cell) showStripTip(cell);
      }
    } catch {
      // the tooltip simply stays without a name
    }
  }, 120);
}

function openCell(i) {
  const info = strip.info.get(i * strip.group + 1);
  if (strip.group !== 1 || !info) return;
  state.list.q = info.id || info.url;
  state.list.filter = 'all';
  $('#links-q').value = state.list.q;
  location.hash = '#links';
}

function wireStrip() {
  const canvas = $('#strip');
  canvas.addEventListener('pointermove', (e) => {
    const i = cellAt(e);
    strip.hover = i;
    showStripTip(i);
  });
  canvas.addEventListener('pointerleave', () => {
    strip.hover = -1;
    if (strip.focus < 0) showStripTip(-1);
  });
  canvas.addEventListener('click', (e) => openCell(cellAt(e)));
  canvas.addEventListener('focus', () => {
    if (strip.focus < 0) strip.focus = 0;
    strip.hover = strip.focus;
    drawStrip();
    showStripTip(strip.focus);
  });
  canvas.addEventListener('blur', () => {
    strip.focus = -1;
    drawStrip();
    showStripTip(-1);
  });
  canvas.addEventListener('keydown', (e) => {
    const moves = { ArrowRight: 1, ArrowLeft: -1, ArrowDown: strip.cols, ArrowUp: -strip.cols };
    let next = strip.focus;
    if (e.key in moves) next += moves[e.key];
    else if (e.key === 'Home') next = 0;
    else if (e.key === 'End') next = strip.cells - 1;
    else if (e.key === 'Enter') {
      openCell(strip.focus);
      return;
    } else return;
    e.preventDefault();
    strip.focus = Math.min(Math.max(next, 0), strip.cells - 1);
    strip.hover = strip.focus;
    drawStrip();
    showStripTip(strip.focus);
  });
  new ResizeObserver(() => drawStrip()).observe($('#strip-area'));
}

/* ------------------------------------------------------- per-day columns */

function renderBars() {
  const daily = state.status.daily || [];
  const sig = `${lang}|${daily.join(',')}`;
  if (sig === state.barsSig) return;
  state.barsSig = sig;

  const max = Math.max(0, ...daily);
  const last = daily.length - 1;
  const peak = daily.indexOf(max);
  const today = new Date(serverNow() * 1000);
  const rows = [];
  const slots = daily.map((n, i) => {
    const day = new Date(today);
    day.setDate(today.getDate() - (last - i));
    const label = `${fmt.day.format(day)}: ${t('ov.dayCount', { n })}`;
    rows.push(h('tr', {}, h('th', { scope: 'row', text: fmt.day.format(day) }), h('td', { text: num(n) })));
    const slot = h('div', { class: 'bar-slot', tabindex: '0', 'aria-label': label, 'data-tip': label });
    const height = max ? `${(n / max) * 100}%` : '0%';
    const col = h('div', { class: n ? 'bar-col' : 'bar-col zero' });
    col.style.setProperty('--h', height);
    slot.append(col);
    // Labels only where they say something: today, and the busiest day.
    if (n && (i === last || i === peak)) {
      const value = h('span', { class: 'bar-label', 'aria-hidden': 'true', text: num(n) });
      value.style.setProperty('--h', height);
      slot.append(value);
    }
    return slot;
  });
  $('#bars').replaceChildren(...slots);
  $('#bars-empty').hidden = max > 0;
  $('#bars-table').replaceChildren(h('caption', { text: t('ov.perDay') }), h('tbody', {}, rows));
}

function wireFloatingTips() {
  const tip = $('#tip');
  const show = (target) => {
    const text = target?.dataset?.tip;
    if (!text) {
      tip.hidden = true;
      return;
    }
    tip.textContent = text;
    tip.hidden = false;
    const rect = target.getBoundingClientRect();
    const half = tip.offsetWidth / 2;
    tip.style.left = `${Math.min(Math.max(rect.left + rect.width / 2, half + 8), window.innerWidth - half - 8)}px`;
    tip.style.top = `${rect.top + 12}px`;
  };
  const bars = $('#bars');
  bars.addEventListener('pointerover', (e) => show(e.target.closest('[data-tip]')));
  bars.addEventListener('pointerleave', () => show(null));
  bars.addEventListener('focusin', (e) => show(e.target.closest('[data-tip]')));
  bars.addEventListener('focusout', () => show(null));
}

/* ----------------------------------------------------------- latest/setup */

async function renderLatest() {
  const s = state.status;
  const sig = `${lang}|${s.total}|${s.downloads.done}|${s.downloads.failed}|${s.downloads.no_media}`;
  if (sig === state.latestSig) return;
  state.latestSig = sig;
  try {
    const data = await getJSON('/api/links?limit=6');
    const items = data.items.map((item) => h('li', {},
      h('a', { class: 'handle', href: item.url, target: '_blank', rel: 'noopener noreferrer' },
        who(item), item.id ? h('span', { class: 'id', text: item.id }) : null),
      h('span', { class: 'when', text: item.captured ? ago(item.captured) : '' }),
      stateChip(item.status)));
    $('#latest').replaceChildren(...(items.length ? items : [h('li', { class: 'empty', text: t('ov.latestEmpty') })]));
  } catch {
    state.latestSig = '';
  }
}

function extensionFolder() {
  const dir = state.tools?.tools_dir;
  if (!dir) return null;
  const sep = dir.includes('\\') && !dir.includes('/') ? '\\' : '/';
  return `${dir.replace(/[\\/]+$/, '')}${sep}extension`;
}

/** Whether version `a` comes before `b`, number by number: 1.4.10 is after 1.4.9. */
function olderVersion(a, b) {
  const x = String(a).split('.').map(Number);
  const y = String(b).split('.').map(Number);
  for (let i = 0; i < Math.max(x.length, y.length); i += 1) {
    const d = (x[i] || 0) - (y[i] || 0);
    if (d) return d < 0;
  }
  return false;
}

function renderSetup() {
  const s = state.status;
  const tools = state.tools;
  const items = [];
  if (!s.extension_seen) {
    const folder = extensionFolder();
    items.push(h('li', {},
      h('strong', { text: t('setup.ext') }),
      h('p', { text: t('setup.extHow') }),
      folder ? h('div', { class: 'copy-line' },
        h('code', { text: folder }),
        h('button', { type: 'button', class: 'btn small', text: t('copy'), onclick: () => copyText(folder) })) : null));
  }
  if (tools && !tools.python) {
    items.push(h('li', {}, h('strong', { text: t('setup.python') }), h('p', { text: t('setup.pythonHow') })));
  } else if (tools && tools.downloader && !tools.yt_dlp && !tools.gallery_dl) {
    items.push(h('li', {},
      h('strong', { text: t('setup.tools') }),
      h('p', { text: t('setup.toolsHow') }),
      h('div', {}, h('button', {
        type: 'button',
        class: 'btn primary',
        text: t('setup.install'),
        disabled: isActive(s.job),
        onclick: installTools,
      }))));
  }
  // Only an update left to do is upkeep, not setup; the heading says which.
  const setupItems = items.length;
  const ext = s.extension_version;
  if (s.extension_seen && ext && olderVersion(ext, s.version)) {
    items.push(h('li', {},
      h('strong', { text: t('setup.extOld') }),
      h('p', { text: ext === '0' ? t('setup.extOldEarlier', { app: s.version }) : t('setup.extOldHow', { ext, app: s.version }) }),
      h('p', { text: t('setup.extOldReload', { app: s.version }) })));
  }
  const age = toolAge(tools?.yt_dlp);
  if (tools?.yt_dlp && (state.suggestUpdate || age > STALE_DAYS)) {
    items.push(h('li', {},
      h('strong', { text: t('setup.update') }),
      h('p', { text: state.suggestUpdate ? t('setup.updateFailing') : t('setup.updateOld', { days: age }) }),
      h('div', {}, h('button', {
        type: 'button',
        class: 'btn primary',
        text: t('setup.updateNow'),
        disabled: isActive(s.job),
        onclick: installTools,
      }))));
  }
  const sig = `${lang}|${!s.extension_seen}|${ext}|${extensionFolder()}|${tools?.python}|${tools?.yt_dlp}|${tools?.gallery_dl}|${isActive(s.job)}|${state.suggestUpdate}|${age}`;
  if (sig === state.setupSig) return;
  state.setupSig = sig;
  $('#setup').hidden = items.length === 0;
  $('#setup-title').textContent = t(setupItems ? 'setup.title' : 'setup.titleUpkeep');
  $('#setup-items').replaceChildren(...items);
}

/** yt-dlp versions are dates: 2026.08.19. Days since then, or 0 when unknown. */
const STALE_DAYS = 60;
function toolAge(version) {
  const m = /^(\d{4})\.(\d{2})\.(\d{2})/.exec(version || '');
  if (!m) return 0;
  const released = Date.UTC(Number(m[1]), Number(m[2]) - 1, Number(m[3])) / 1000;
  return Math.max(0, Math.floor((serverNow() - released) / 86400));
}

function installTools() {
  startJob({ kind: 'setup' });
  if (state.view === 'overview') location.hash = '#settings';
}

/* ----------------------------------------------------------------- links */

const FILTERS = ['all', 'pending', 'done', 'no-media', 'failed', 'gone'];

function renderFilters() {
  const d = state.status?.downloads;
  const counts = d ? { all: state.status.total, pending: d.pending, done: d.done, 'no-media': d.no_media, failed: d.failed, gone: d.gone || 0 } : {};
  // "Moved away" only once there is something to show under it.
  const shown = FILTERS.filter((key) => key !== 'gone' || counts.gone || state.list.filter === 'gone');
  $('#links-filter').replaceChildren(...shown.map((key) => h('button', {
    type: 'button',
    'aria-pressed': String(state.list.filter === key),
    onclick: () => {
      state.list.filter = key;
      renderFilters();
      loadLinks(true);
    },
  }, key === 'all' ? t('links.all') : key === 'gone' ? t('links.gone') : t(`state.${key}`), key in counts ? h('span', { class: 'count', text: num(counts[key]) }) : null)));
  $('#links-file').textContent = state.status?.links_file || '';
}

/** Reload the first page when the numbers move, unless the reader is busy in the list. */
function renderLinksMeta(before) {
  const s = state.status;
  const d = s.downloads;
  const sig = `${s.total}|${d.done}|${d.no_media}|${d.failed}|${d.gone}`;
  if (state.list.sig !== sig) {
    renderFilters();
    const interacting = $('#view-links').contains(document.activeElement) && document.activeElement !== document.body
      && !$('#links-filter').contains(document.activeElement);
    if (before && !interacting && state.list.items.length <= 50) loadLinks(true);
  }
}

async function loadLinks(reset) {
  const list = state.list;
  if (list.busy) return;
  list.busy = true;
  const offset = reset ? 0 : list.items.length;
  const query = new URLSearchParams({ q: list.q, status: list.filter, offset: String(offset), limit: '50' });
  try {
    const data = await getJSON(`/api/links?${query}`);
    list.items = reset ? data.items : list.items.concat(data.items);
    list.matched = data.matched;
    list.total = data.total;
    const d = state.status?.downloads;
    if (state.status && d) list.sig = `${state.status.total}|${d.done}|${d.no_media}|${d.failed}|${d.gone}`;
    renderLinks();
  } catch (e) {
    toast(explain(e), 'error');
  } finally {
    list.busy = false;
  }
}

function renderLinks() {
  const list = state.list;
  $('#links-list').replaceChildren(...list.items.map(linkRow));
  const empty = $('#links-empty');
  empty.hidden = list.items.length > 0;
  empty.textContent = list.total === 0 ? t('links.empty') : t('links.noMatch');
  $('#links-meta').textContent = list.items.length ? t('links.shown', { shown: list.items.length, matched: list.matched }) : '';
  const more = $('#links-more');
  more.hidden = list.items.length >= list.matched;
  more.textContent = t('links.more');
}

const isVideo = (file) => /\.(mp4|m4v|mov|webm|mkv)$/i.test(file);
const isImage = (file) => /\.(jpe?g|png|webp|gif)$/i.test(file);

/** x-download records paths relative to the media folder, \ on Windows. */
function mediaUrl(file) {
  return `/media/${file.replace(/\\/g, '/').split('/').map(encodeURIComponent).join('/')}`;
}

/** Video thumbnails load as they scroll into view: a page of links is fifty files. */
const lazyVideos = 'IntersectionObserver' in window ? new IntersectionObserver((entries) => {
  for (const entry of entries) {
    if (!entry.isIntersecting) continue;
    entry.target.src = entry.target.dataset.src;
    lazyVideos.unobserve(entry.target);
  }
}, { rootMargin: '200px' }) : null;

function thumb(item) {
  // Moved away: there is nothing at those paths to show.
  const files = item.gone ? [] : (item.files || []).filter((f) => isVideo(f) || isImage(f));
  if (!files.length) return h('span', { class: 'thumb empty', 'aria-hidden': 'true' });
  const button = h('button', {
    type: 'button',
    class: 'thumb',
    'aria-label': t('links.show'),
    onclick: () => openViewer(item, files, 0),
  });
  const first = files[0];
  if (isImage(first)) {
    button.append(h('img', { src: mediaUrl(first), alt: '', loading: 'lazy', decoding: 'async' }));
  } else {
    // #t= asks for a frame a moment in, rather than a black first one.
    const video = h('video', { muted: true, preload: 'metadata', playsinline: true, 'aria-hidden': 'true', 'data-src': `${mediaUrl(first)}#t=0.5` });
    if (lazyVideos) lazyVideos.observe(video);
    else video.src = video.dataset.src;
    button.append(video, icon('play'));
  }
  if (files.length > 1) button.append(h('span', { class: 'more', text: `+${files.length - 1}` }));
  return button;
}

/* ---------------------------------------------------------------- gallery */

const gallery = { kind: '', handle: '', items: [], matched: 0, total: 0, handles: [], busy: false };
const KINDS = ['', 'video', 'image'];

async function loadGallery(reset) {
  if (gallery.busy) return;
  gallery.busy = true;
  try {
    const query = new URLSearchParams({ kind: gallery.kind, handle: gallery.handle, offset: reset ? 0 : gallery.items.length, limit: 60 });
    const page = await getJSON(`/api/gallery?${query}`);
    gallery.items = reset ? page.items : gallery.items.concat(page.items);
    Object.assign(gallery, { matched: page.matched, total: page.total, handles: page.handles });
    renderGallery(reset);
  } catch (e) {
    toast(explain(e), 'error');
  } finally {
    gallery.busy = false;
  }
}

function renderGallery(reset) {
  $('#ga-kind').replaceChildren(...KINDS.map((kind) => h('button', {
    type: 'button',
    'aria-pressed': String(gallery.kind === kind),
    text: t(kind ? `ga.kind.${kind}` : 'links.all'),
    onclick: () => {
      gallery.kind = kind;
      loadGallery(true);
    },
  })));
  const select = $('#ga-handle');
  select.replaceChildren(
    h('option', { value: '', text: t('ga.everyone', { n: gallery.total }) }),
    ...gallery.handles.map((a) => h('option', { value: a.handle, text: `${a.handle ? `@${a.handle}` : t('ga.loose')} (${num(a.count)})` })));
  select.value = gallery.handle;

  $('#ga-meta').textContent = gallery.matched ? t('links.shown', { shown: gallery.items.length, matched: gallery.matched }) : '';
  const grid = $('#ga-grid');
  const tiles = (reset ? gallery.items : gallery.items.slice(grid.children.length)).map(galleryTile);
  if (reset) grid.replaceChildren(...tiles);
  else grid.append(...tiles);
  const empty = !gallery.items.length;
  $('#ga-empty').hidden = !empty;
  $('#ga-empty').textContent = gallery.total ? t('links.noMatch') : t('ga.empty');
  $('#ga-more').hidden = gallery.items.length >= gallery.matched;
}

function galleryTile(file) {
  const item = { id: file.id, handle: file.handle, url: file.id ? `https://x.com/${file.handle || 'i/web'}/status/${file.id}` : '' };
  // The viewer steps through this tweet's files, as it does from the Links page.
  const open = () => {
    const files = gallery.items.filter((f) => f.id && f.id === file.id).map((f) => f.path);
    const list = files.length ? files : [file.path];
    openViewer(item, list, Math.max(0, list.indexOf(file.path)));
  };
  const button = h('button', { type: 'button', class: 'tile', 'aria-label': `${t('links.show')}: ${who(item)}`, onclick: open });
  if (file.kind === 'image') {
    button.append(h('img', { src: mediaUrl(file.path), alt: '', loading: 'lazy', decoding: 'async' }));
  } else {
    const video = h('video', { muted: true, preload: 'metadata', playsinline: true, 'aria-hidden': 'true', 'data-src': `${mediaUrl(file.path)}#t=0.5` });
    if (lazyVideos) lazyVideos.observe(video);
    else video.src = video.dataset.src;
    button.append(video, icon('play'));
  }
  button.append(h('span', { class: 'tile-who', text: who(item) }));
  return h('li', {}, button);
}

function wireGallery() {
  $('#ga-handle').addEventListener('change', (e) => {
    gallery.handle = e.target.value;
    loadGallery(true);
  });
  $('#ga-more').addEventListener('click', () => loadGallery(false));
}

/* ----------------------------------------------------------------- viewer */

const viewer = { item: null, files: [], index: 0 };

function openViewer(item, files, index) {
  Object.assign(viewer, { item, files, index });
  $('#viewer-title').textContent = who(item);
  const open = $('#viewer-open');
  open.href = item.url;
  open.textContent = t(item.id ? 'links.open' : 'links.openOther');
  showViewerFile();
  $('#viewer').showModal();
}

function showViewerFile() {
  const file = viewer.files[viewer.index];
  $('#viewer-stage').replaceChildren(isVideo(file)
    ? h('video', { src: mediaUrl(file), controls: true, autoplay: true, playsinline: true })
    : h('img', { src: mediaUrl(file), alt: who(viewer.item) }));
  const many = viewer.files.length > 1;
  $('#viewer-count').textContent = many ? t('viewer.count', { i: viewer.index + 1, n: viewer.files.length }) : '';
  $('#viewer-prev').hidden = !many;
  $('#viewer-next').hidden = !many;
}

function stepViewer(by) {
  viewer.index = (viewer.index + by + viewer.files.length) % viewer.files.length;
  showViewerFile();
}

function wireViewer() {
  const dialog = $('#viewer');
  $('#viewer-close').addEventListener('click', () => dialog.close());
  $('#viewer-prev').addEventListener('click', () => stepViewer(-1));
  $('#viewer-next').addEventListener('click', () => stepViewer(1));
  // Closing must also stop a playing video.
  dialog.addEventListener('close', () => $('#viewer-stage').replaceChildren());
  dialog.addEventListener('keydown', (e) => {
    if (viewer.files.length < 2) return;
    if (e.key === 'ArrowLeft') stepViewer(-1);
    if (e.key === 'ArrowRight') stepViewer(1);
  });
  dialog.addEventListener('click', (e) => {
    if (e.target === dialog) dialog.close(); // a click on the dimmed backdrop
  });
}

function downloadOne(item, next) {
  const body = next === 'again'
    ? { kind: 'download', again: [item.url] }
    : { kind: 'download', only: [item.url], retry_failed: next === 'retry' };
  startJob(body, t('job.startedDownload'));
}

function linkRow(item) {
  const posted = item.id ? postedAt(item.id) : null;
  // A row's own next step: fetch what is waiting, retry what did not come
  // down, and fetch once more what was downloaded but has since moved away.
  const next = item.status === 'pending' ? 'download'
    : item.status !== 'done' ? 'retry'
      : item.gone ? 'again' : null;
  return h('li', { class: 'link-row' },
    thumb(item),
    h('span', { class: 'line', text: t('links.line', { n: item.line }) }),
    h('div', { class: 'who' },
      h('a', { class: 'handle', href: item.url, target: '_blank', rel: 'noopener noreferrer' },
        who(item), item.id ? h('span', { class: 'id', text: item.id }) : null),
      h('span', { class: 'url', text: item.url })),
    h('div', { class: 'dates' },
      posted ? h('span', { text: t('links.posted', { date: fmt.date.format(posted) }) }) : null,
      item.captured ? h('span', { text: t('links.collected', { ago: ago(item.captured) }) }) : null),
    h('div', { class: 'status' },
      stateChip(item.status),
      item.gone ? h('span', { class: 'moved', text: t('links.moved') }) : null),
    h('div', { class: 'actions' },
      next ? h('button', {
        type: 'button',
        class: 'btn small quiet',
        text: t(`links.${next}`),
        disabled: isActive(state.status?.job),
        onclick: () => downloadOne(item, next),
      }) : null,
      h('a', { class: 'btn small quiet', href: item.url, target: '_blank', rel: 'noopener noreferrer', text: t(item.id ? 'links.open' : 'links.openOther') }),
      h('button', { type: 'button', class: 'btn small quiet', text: t('links.copy'), onclick: () => copyText(item.url) }),
      h('button', { type: 'button', class: 'btn small quiet danger', text: t('links.remove'), onclick: () => removeLink(item) })),
    item.status === 'failed' && item.error ? whyFailed(item.error) : null);
}

async function removeLink(item) {
  const sure = await confirmAction({ title: t('links.removeTitle'), body: `${item.url}\n\n${t('links.removeBody')}`, action: t('links.remove') });
  if (!sure) return;
  try {
    await postJSON('/api/links/remove', { url: item.url });
    toast(t('links.removed'));
    await loadLinks(true);
    kick();
  } catch (e) {
    toast(explain(e), 'error');
  }
}

function wireLinks() {
  let typing = 0;
  $('#links-q').addEventListener('input', (e) => {
    clearTimeout(typing);
    typing = setTimeout(() => {
      state.list.q = e.target.value.trim();
      loadLinks(true);
    }, 250);
  });
  const box = $('#links-add');
  const toggle = (open) => {
    box.hidden = !open;
    $('#links-add-open').setAttribute('aria-expanded', String(open));
    if (open) $('#links-add-text').focus();
  };
  $('#links-add-open').addEventListener('click', () => toggle(box.hidden));
  $('#links-add-close').addEventListener('click', () => toggle(false));
  box.addEventListener('submit', async (e) => {
    e.preventDefault();
    const text = $('#links-add-text').value.trim();
    if (!text) return;
    try {
      const r = await postJSON('/api/links', { urls: [text], downloaded: $('#links-add-done').checked });
      // Only the parts that happened: "0 were already there" says nothing.
      const said = [];
      if (r.added) said.push(t('links.addedNew', { n: r.added }));
      if (r.duplicate) said.push(t('links.addedKnown', { n: r.duplicate }));
      if (r.marked) said.push(t('links.marked', { n: r.marked }));
      toast(said.join(', '));
      if (r.rejected) toast(t('links.rejected', { n: r.rejected }), 'error');
      $('#links-add-text').value = '';
      toggle(false);
      await loadLinks(true);
      kick();
    } catch (err) {
      toast(explain(err), 'error');
    }
  });
  $('#links-more').addEventListener('click', () => loadLinks(false));
  $('#links-import').addEventListener('change', (e) => importArchive(e.target));
  wireExport();
}

/** Tweets in an X archive file (like.js, bookmark.js), a links.txt, or any text with links. */
function archiveLinks(text) {
  // An archive names each tweet by id; its text may quote other tweets,
  // which were not liked, so those links count only when there are no ids.
  const ids = [...text.matchAll(/"tweetId"\s*:\s*"(\d{1,25})"/g)].map((m) => m[1]);
  if (ids.length) return [...new Set(ids)].map((id) => `https://x.com/i/web/status/${id}`);
  const links = text.match(/https?:\/\/(?:www\.|mobile\.)?(?:x|twitter)\.com\/(?:[A-Za-z0-9_]{1,15}|i\/web)\/status(?:es)?\/\d{1,25}/g) || [];
  // A links.txt may also hold bare tweet ids, a line each, as x-download reads them.
  const bare = [...text.matchAll(/^[ \t]*(\d{5,25})[ \t]*\r?$/gm)].map((m) => `https://x.com/i/web/status/${m[1]}`);
  return [...new Set([...links, ...bare])];
}

async function importArchive(input) {
  const file = input.files?.[0];
  input.value = '';
  if (!file) return;
  toast(t('links.importing', { file: file.name }));
  try {
    const links = archiveLinks(await file.text());
    if (!links.length) {
      toast(t('links.importNone', { file: file.name }), 'error');
      return;
    }
    toast(t('links.imported', { file: file.name, n: links.length }));
    let added = 0;
    let duplicate = 0;
    let marked = 0;
    const downloaded = $('#links-add-done').checked;
    // A few thousand at a time keeps each request well under the body limit.
    for (let i = 0; i < links.length; i += 5000) {
      const r = await postJSON('/api/links', { urls: [links.slice(i, i + 5000).join('\n')], imported: true, downloaded });
      added += r.added || 0;
      duplicate += r.duplicate || 0;
      marked += r.marked || 0;
    }
    const said = [];
    if (added) said.push(t('links.addedNew', { n: added }));
    if (duplicate) said.push(t('links.addedKnown', { n: duplicate }));
    if (marked) said.push(t('links.marked', { n: marked }));
    toast(said.join(', '));
    await loadLinks(true);
    kick();
  } catch (err) {
    toast(explain(err), 'error');
  }
}

/** The Export menu downloads what the list shows: this search, this filter. */
function wireExport() {
  const menu = $('#links-export');
  const href = (format) => `/api/export?${new URLSearchParams({ format, q: state.list.q, status: state.list.filter })}`;
  menu.addEventListener('toggle', () => {
    if (!menu.open) return;
    // Open towards the side with room: the toolbar wraps on narrow windows.
    const box = menu.getBoundingClientRect();
    menu.classList.toggle('end', box.left + 280 > document.documentElement.clientWidth);
    $('#links-export-csv').href = href('csv');
    $('#links-export-json').href = href('json');
  });
  for (const a of menu.querySelectorAll('a')) a.addEventListener('click', () => { menu.open = false; });
  document.addEventListener('click', (e) => { if (menu.open && !menu.contains(e.target)) menu.open = false; });
  menu.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && menu.open) {
      menu.open = false;
      menu.querySelector('summary').focus();
    }
  });
}

/* -------------------------------------------------------------- download */

const ROUTES = ['off', 'fallback', 'only'];
const BROWSERS = ['brave', 'chrome', 'chromium', 'edge', 'firefox', 'opera', 'safari', 'vivaldi'];

/** The login picker, on the Download page and in Settings: a browser, or a cookies.txt path. */
function fillCookies(select, path, current) {
  const isFile = Boolean(current) && !BROWSERS.includes(current.toLowerCase());
  select.replaceChildren(
    h('option', { value: '', text: t('cookies.none') }),
    ...BROWSERS.map((b) => h('option', { value: b, text: b[0].toUpperCase() + b.slice(1) })),
    h('option', { value: '@file', text: t('cookies.file') }));
  select.value = isFile ? '@file' : (current || '').toLowerCase();
  path.hidden = !isFile;
  path.value = isFile ? current : '';
}

/*
 * Polled every couple of seconds, so it only touches what changed: rebuilding
 * a button or a select under the reader would take their focus away.
 */
function renderDownload() {
  renderGoButton($('#dl-go'));
  const s = state.settings?.settings;
  if (!s) return;
  const active = document.activeElement;

  if (state.dlLang !== lang) {
    state.dlLang = lang;
    state.dlCookies = null;
    $('#dl-route').replaceChildren(...ROUTES.map((key) => h('button', {
      type: 'button',
      'data-route': key,
      onclick: () => saveNow({ mirror: key }),
    }, t(`route.${key}`))));
  }
  for (const button of $('#dl-route').children) {
    button.setAttribute('aria-pressed', String(button.dataset.route === s.mirror));
  }
  $('#dl-route-help').textContent = t(`route.${s.mirror}Help`);

  if (active !== $('#dl-jobs')) $('#dl-jobs').value = s.jobs;

  const select = $('#dl-cookies');
  const path = $('#dl-cookies-path');
  const cookies = `${lang}|${s.cookies}`;
  if (state.dlCookies !== cookies && active !== select && active !== path) {
    state.dlCookies = cookies;
    fillCookies(select, path, s.cookies);
  }

  $('#dl-auto').checked = Boolean(s.auto_download);
  renderAgain();
}

function renderAgain() {
  const d = state.status?.downloads;
  if (!d) return;
  const remember = d.remember !== false;
  $('#dl-again-lead').textContent = t(remember ? 'again.leadRemember' : 'again.leadForget');
  const gone = d.gone || 0;
  $('#dl-again-gone').hidden = gone === 0;
  if (!gone) return;
  $('#dl-again-gone-text').textContent = t(remember ? 'again.gone' : 'again.goneForget', { n: gone });
  const go = $('#dl-again-gone-go');
  go.textContent = t('again.goneGo', { n: gone });
  go.disabled = isActive(state.status.job);
  $('#dl-again-go').disabled = isActive(state.status.job);
}

async function saveNow(changes) {
  try {
    state.settings = await postJSON('/api/settings', changes);
    for (const key of Object.keys(changes)) delete state.draft[key];
    toast(t('dl.saved'));
    renderView();
    kick();
  } catch (e) {
    toast(explain(e), 'error');
    renderView();
  }
}

function wireDownload() {
  const jobsInput = $('#dl-jobs');
  const setJobs = (n) => {
    const value = Math.min(10, Math.max(1, n));
    jobsInput.value = value;
    if (value !== state.settings?.settings?.jobs) saveNow({ jobs: value });
  };
  $('#dl-jobs-less').addEventListener('click', () => setJobs(parseInt(jobsInput.value, 10) - 1));
  $('#dl-jobs-more').addEventListener('click', () => setJobs(parseInt(jobsInput.value, 10) + 1));
  jobsInput.addEventListener('change', () => setJobs(parseInt(jobsInput.value, 10) || 1));

  $('#dl-cookies').addEventListener('change', (e) => {
    const value = e.target.value;
    if (value === '@file') {
      $('#dl-cookies-path').hidden = false;
      $('#dl-cookies-path').focus();
      return;
    }
    saveNow({ cookies: value });
  });
  $('#dl-cookies-path').addEventListener('change', (e) => saveNow({ cookies: e.target.value.trim() }));
  $('#dl-auto').addEventListener('change', (e) => saveNow({ auto_download: e.target.checked }));

  $('#dl-go').addEventListener('click', (e) => {
    startJob(downloadBody(e.currentTarget.dataset.retry ? { retry_failed: true } : {}), t('job.startedDownload'));
  });
  $('#dl-preview').addEventListener('click', () => startJob(downloadBody({ dry_run: true }), t('job.startedPreview')));

  $('#dl-again-gone-go').addEventListener('click', async () => {
    const n = state.status?.downloads?.gone || 0;
    const sure = await confirmAction({
      title: t('again.goneTitle', { n }),
      body: t('again.goneBody', { dir: state.status.media_dir }),
      action: t('again.goneGo', { n }),
    });
    if (sure) startJob({ kind: 'download', again_missing: true }, t('job.startedDownload'));
  });
  $('#dl-again-gone-show').addEventListener('click', () => {
    state.list.filter = 'gone';
    location.hash = '#links';
  });
  $('#dl-again-form').addEventListener('submit', (e) => {
    e.preventDefault();
    const links = $('#dl-again-text').value.split(/\s+/).map((l) => l.trim()).filter(Boolean);
    if (!links.length) {
      toast(t('again.none'), 'error');
      return;
    }
    startJob({ kind: 'download', again: links }, t('job.startedDownload'));
    $('#dl-again-text').value = '';
  });
  $('#ov-go').addEventListener('click', (e) => {
    startJob({ kind: 'download', retry_failed: Boolean(e.currentTarget.dataset.retry) }, t('job.startedDownload'));
  });
  $('#ov-stop').addEventListener('click', stopJob);
  $('#job-stop').addEventListener('click', stopJob);
}

async function stopJob() {
  try {
    await postJSON('/api/job/stop');
    kick();
  } catch (e) {
    toast(explain(e), 'error');
  }
}

/* -------------------------------------------------------------- job panel */

function renderJob() {
  const panel = $('#job');
  // Shown on the pages that start jobs; the overview has its own progress.
  panel.hidden = !document.querySelector(`#view-${state.view} .job-slot`);
  if (panel.hidden) return;
  const job = state.job;
  const log = $('#job-log');

  if (!job) {
    $('#job-title').textContent = t('job.output');
    $('#job-meta').textContent = '';
    $('#job-stop').hidden = true;
    $('#job-progress').hidden = true;
    if (!log.childElementCount) log.append(h('span', { class: 'idle', text: t('job.idle') }));
    return;
  }

  $('#job-title').textContent = t(`job.title.${job.kind}`);
  const active = isActive(job);
  $('#job-stop').hidden = !active;
  $('#job-stop').disabled = job.state === 'stopping';

  const parts = [t(`job.state.${job.state}`)];
  if (job.total) parts.push(t('job.progress', { done: job.done, total: job.total }));
  if (active && job.total && job.done) {
    const elapsed = serverNow() - job.started;
    parts.push(t('job.left', { t: duration((elapsed / job.done) * (job.total - job.done)) }));
  }
  // How it ended is the log's last line, just below.
  $('#job-meta').textContent = parts.join(', ');

  const progress = $('#job-progress');
  progress.hidden = !active && !job.total;
  progress.classList.toggle('busy', active && !job.total);
  const share = job.total ? (job.done / job.total) * 100 : job.state === 'done' ? 100 : 0;
  $('#job-fill').style.setProperty('--p', `${share}%`);

  const idle = log.querySelector('.idle');
  if (idle) idle.remove();
  if (state.shown < state.lines.length) {
    const atBottom = log.scrollTop + log.clientHeight >= log.scrollHeight - 32;
    const batch = document.createDocumentFragment();
    for (const line of state.lines.slice(state.shown)) batch.append(h('span', { class: lineKind(line), text: `${lineText(line)}\n` }));
    state.shown = state.lines.length;
    log.append(batch);
    if (atBottom) log.scrollTop = log.scrollHeight;
  }
}

/** The receiver's own closing line, in the reader's language; the tools' output stays as they wrote it. */
function lineText(line) {
  const end = line.text.startsWith('-- ') ? line.text.slice(3) : null;
  if (end === 'finished') return `-- ${t('job.state.done')}`;
  if (end === 'stopped') return `-- ${t('job.state.stopped')}`;
  const exited = end && /^exited with code (-?\d+)$/.exec(end);
  if (exited) return `-- ${t('job.exited', { code: exited[1] })}`;
  return line.text;
}

function lineKind(line) {
  if (line.err) return 'err';
  if (line.text.startsWith('$ ')) return 'cmd';
  if (line.text.startsWith('-- ')) return 'end';
  if (/^\[\s*\d+\/\d+\]\s+ok\b/.test(line.text)) return 'ok';
  return '';
}

/* ---------------------------------------------------------------- folder */

function renderFolder(first) {
  const s = state.status;
  const view = state.settings;
  if (!s) return;
  $('#fo-path').textContent = s.media_dir;
  const m = s.media;
  const count = $('#fo-count');
  if (m.files) {
    count.replaceChildren(
      t('fo.files', { n: m.files, size: bytes(m.bytes) }),
      h('span', { class: 'sub', text: t('fo.kinds', {
        videos: t('fo.kindVideos', { n: m.videos }),
        images: t('fo.kindImages', { n: m.images }),
      }) }));
  } else {
    count.textContent = t('fo.none');
  }
  $('#fo-undo').hidden = !m.flattened;
  const busy = isActive(s.job);
  for (const id of ['#fo-preview', '#fo-go', '#fo-undo']) $(id).disabled = busy;
  if (first && view) {
    const set = view.settings;
    $('#fo-dest').value = set.flatten_dest;
    $('#fo-dest').placeholder = view.effective.flatten_dest;
    $('#fo-prefix').checked = set.flatten_prefix_handle;
    $('#fo-videos').checked = set.flatten_videos_only;
    $('#fo-copy').checked = set.flatten_copy;
  }
}

function flattenBody(extra) {
  return {
    kind: 'flatten',
    dest: $('#fo-dest').value.trim(),
    prefix_handle: $('#fo-prefix').checked,
    videos_only: $('#fo-videos').checked,
    copy: $('#fo-copy').checked,
    ...extra,
  };
}

function wireFolder() {
  $('#fo-open').addEventListener('click', () => openThing('media'));
  $('#fo-preview').addEventListener('click', () => startJob(flattenBody({ dry_run: true })));
  $('#fo-go').addEventListener('click', async () => {
    const dest = $('#fo-dest').value.trim() || $('#fo-dest').placeholder;
    const sure = await confirmAction({ title: t('fo.flattenTitle'), body: t('fo.flattenBody', { dest }), action: t('fo.flatten') });
    if (sure) startJob(flattenBody({}));
  });
  $('#fo-undo').addEventListener('click', async () => {
    const sure = await confirmAction({ title: t('fo.undoTitle'), body: t('fo.undoBody'), action: t('fo.undo') });
    if (sure) startJob({ kind: 'unflatten', dest: $('#fo-dest').value.trim() });
  });
}

async function openThing(what) {
  try {
    await postJSON('/api/open', { what });
  } catch (e) {
    toast(explain(e), 'error');
  }
}

/* -------------------------------------------------------------- settings */

/*
 * The form is built from this list. Values are edited into `state.draft` and
 * saved together; settings fixed on the command line show as read-only.
 */
const FORM = [
  ['st.files', [
    { key: 'links_file', kind: 'path', placeholder: (v) => v.defaults.links_file, open: 'links' },
    { key: 'media_dir', kind: 'path', placeholder: (v) => v.defaults.media_dir, open: 'media' },
  ]],
  ['st.downloading', [
    { key: 'mirror', kind: 'choice', options: ROUTES, label: 'st.mirror', optionLabel: (k) => t(`route.${k}`), help: (v) => t(`route.${v}Help`) },
    { key: 'jobs', kind: 'number', min: 1, max: 10, label: 'st.jobs', help: () => t('dl.jobsHelp') },
    { key: 'cookies', kind: 'cookies', label: 'st.cookies', help: () => t('dl.cookiesHelp') },
    { key: 'timeout', kind: 'number', min: 30, max: 86400, unit: 'st.seconds' },
    { key: 'metadata', kind: 'switch' },
    { key: 'auto_download', kind: 'switch', help: () => t('dl.autoHelp') },
    { key: 'remember_downloads', kind: 'switch' },
    { key: 'link_duplicates', kind: 'switch' },
  ]],
  ['st.flatten', [
    { key: 'flatten_dest', kind: 'path', placeholder: (v) => v.effective.flatten_dest },
    { key: 'flatten_prefix_handle', kind: 'switch' },
    { key: 'flatten_videos_only', kind: 'switch' },
    { key: 'flatten_copy', kind: 'switch' },
  ]],
  ['st.collector', [
    { key: 'autostart', kind: 'autostart' },
    { key: 'port', kind: 'number', min: 1024, max: 65535 },
    { key: 'fsync', kind: 'switch' },
  ]],
  ['st.where', [
    { key: 'tools_dir', kind: 'path', placeholder: () => state.tools?.tools_dir || '', open: 'tools' },
    { key: 'python', kind: 'path', placeholder: () => state.tools?.python || '' },
  ]],
];

async function loadSettings() {
  try {
    state.settings = await getJSON('/api/settings');
    renderChrome();
    renderView();
  } catch {
    // the status loop shows the panel is offline
  }
}

function value(key) {
  return key in state.draft ? state.draft[key] : state.settings.settings[key];
}

function setDraft(key, raw) {
  const saved = state.settings.settings[key];
  if (raw === saved) delete state.draft[key];
  else state.draft[key] = raw;
  $('#savebar').hidden = Object.keys(state.draft).length === 0;
}

function renderSettings() {
  const view = state.settings;
  if (!view) return;
  const form = $('#settings-form');
  if (form.contains(document.activeElement) && document.activeElement !== form) return;
  const pinned = new Set(view.pinned || []);

  form.replaceChildren(...FORM.map(([title, fields]) => h('section', { class: 'settings-group' },
    h('h2', { text: t(title) }),
    // Starting at sign-in is offered only where the receiver can arrange it.
    fields
      .filter((field) => field.kind !== 'autostart' || typeof view.autostart === 'boolean')
      .map((field) => settingRow(field, view, pinned.has(field.key))))));
  $('#savebar').hidden = Object.keys(state.draft).length === 0;
}

function settingRow(field, view, pinned) {
  const id = `set-${field.key}`;
  const labelKey = field.label || `st.${field.key}`;
  const helpText = field.help ? field.help(value(field.key)) : WORDS.en[`st.${field.key}Help`] ? t(`st.${field.key}Help`) : '';
  const help = helpText ? h('p', { class: 'help', id: `${id}-help`, text: helpText }) : null;
  const described = help ? `${id}-help` : null;
  let label = h('label', { class: 'label', for: id, text: t(labelKey) });
  let control;

  if (field.kind === 'autostart') {
    // Not part of the settings file: it is applied the moment it is switched.
    const input = h('input', { type: 'checkbox', role: 'switch', id, 'aria-describedby': described });
    input.checked = view.autostart;
    input.addEventListener('change', async () => {
      try {
        await postJSON('/api/autostart', { on: input.checked });
        state.settings.autostart = input.checked;
        toast(t('dl.saved'));
      } catch (e) {
        input.checked = !input.checked;
        toast(explain(e), 'error');
      }
    });
    control = h('label', { class: 'switch' }, input, h('span', { class: 'knob', 'aria-hidden': 'true' }));
  } else if (field.kind === 'switch') {
    const input = h('input', { type: 'checkbox', role: 'switch', id, disabled: pinned, 'aria-describedby': described });
    input.checked = Boolean(value(field.key));
    input.addEventListener('change', () => setDraft(field.key, input.checked));
    control = h('label', { class: 'switch' }, input, h('span', { class: 'knob', 'aria-hidden': 'true' }));
  } else if (field.kind === 'choice') {
    // Updated in place: re-rendering would take focus from the button just pressed.
    label = h('span', { class: 'label', id: `${id}-label`, text: t(labelKey) });
    control = h('div', { class: 'segmented', role: 'group', 'aria-labelledby': `${id}-label` });
    for (const option of field.options) {
      control.append(h('button', {
        type: 'button',
        'data-option': option,
        'aria-pressed': String(value(field.key) === option),
        disabled: pinned,
        onclick: () => {
          setDraft(field.key, option);
          for (const button of control.children) {
            button.setAttribute('aria-pressed', String(button.dataset.option === option));
          }
          if (help && field.help) help.textContent = field.help(option);
        },
      }, field.optionLabel(option)));
    }
  } else if (field.kind === 'cookies') {
    const select = h('select', { class: 'input', id, disabled: pinned, 'aria-describedby': described });
    const path = h('input', {
      class: 'input mono',
      type: 'text',
      spellcheck: 'false',
      autocomplete: 'off',
      disabled: pinned,
      placeholder: t('dl.cookiesPath'),
      'aria-label': t('dl.cookiesPath'),
    });
    fillCookies(select, path, value(field.key));
    select.addEventListener('change', () => {
      const file = select.value === '@file';
      path.hidden = !file;
      if (file) path.focus();
      setDraft(field.key, file ? path.value.trim() : select.value);
    });
    path.addEventListener('input', () => setDraft(field.key, path.value.trim()));
    control = h('div', { class: 'stack' }, select, path);
  } else if (field.kind === 'number') {
    const input = h('input', { class: 'input', type: 'number', id, min: field.min, max: field.max, inputmode: 'numeric', disabled: pinned, 'aria-describedby': described });
    input.value = value(field.key);
    input.addEventListener('input', () => {
      const n = Number(input.value);
      if (Number.isInteger(n) && n >= field.min && n <= field.max) setDraft(field.key, n);
    });
    control = field.unit ? h('div', { class: 'number-unit' }, input, h('span', { text: t(field.unit) })) : input;
  } else {
    const input = h('input', {
      class: field.kind === 'path' ? 'input mono' : 'input',
      type: 'text',
      id,
      spellcheck: 'false',
      autocomplete: 'off',
      disabled: pinned,
      placeholder: field.placeholder ? field.placeholder(view) : '',
      'aria-describedby': described,
    });
    input.value = value(field.key);
    input.addEventListener('input', () => setDraft(field.key, input.value.trim()));
    control = field.open
      ? h('div', { class: 'with-button' }, input, h('button', { type: 'button', class: 'btn', text: t('st.openFolder'), onclick: () => openThing(field.open) }))
      : input;
  }

  return h('div', { class: 'setting' },
    h('div', { class: 'about' }, label, help),
    h('div', { class: 'control' }, control, pinned ? h('p', { class: 'pinned', text: t('st.pinned') }) : null));
}

async function saveSettings() {
  try {
    state.settings = await postJSON('/api/settings', state.draft);
    state.draft = {};
    $('#savebar').hidden = true;
    toast(t('save.saved'));
    document.activeElement?.blur();
    renderView();
    renderChrome();
    kick();
  } catch (e) {
    toast(explain(e), 'error');
  }
}

/** Fold a backup file into this collection, after saying what is in it. */
async function restoreBackup(input) {
  const file = input.files?.[0];
  input.value = '';
  if (!file) return;
  let backup;
  try {
    backup = JSON.parse(await file.text());
  } catch {
    backup = null;
  }
  if (!backup || backup.format !== 'x-link-collector-backup') {
    toast(t('err.not_backup'), 'error');
    return;
  }
  const made = Date.parse(backup.made);
  const sure = await confirmAction({
    title: t('st.restoreTitle'),
    body: t('st.restoreBody', {
      file: file.name,
      made: Number.isNaN(made) ? '?' : fmt.date.format(made),
      n: (backup.links || []).length,
    }),
    action: t('st.restoreDo'),
  });
  if (!sure) return;
  try {
    const r = await postJSON('/api/restore', { backup, settings: $('#backup-settings').checked });
    toast(t('st.restored', { links: r.links || 0, downloads: r.downloads || 0, captured: r.captured || 0 }));
    if (r.settings) {
      toast(t('st.restoredSettings'));
      await loadSettings();
    }
    kick();
  } catch (e) {
    toast(explain(e), 'error');
  }
}

function wireSettings() {
  $('#backup-file').addEventListener('change', (e) => restoreBackup(e.target));
  $('#save-go').addEventListener('click', saveSettings);
  $('#save-discard').addEventListener('click', () => {
    state.draft = {};
    $('#savebar').hidden = true;
    document.activeElement?.blur();
    renderSettings();
  });
  $('#settings-form').addEventListener('submit', (e) => {
    e.preventDefault();
    if (Object.keys(state.draft).length) saveSettings();
  });
  $('#tools-check').addEventListener('click', () => loadTools(true));
  $('#quit').addEventListener('click', async () => {
    const sure = await confirmAction({ title: t('st.quitTitle'), body: t('st.quitHelp'), action: t('st.quit') });
    if (!sure) return;
    try {
      await postJSON('/api/quit');
      toast(t('st.quitDone'));
    } catch (e) {
      toast(explain(e), 'error');
    }
  });
  $('#tools-install').addEventListener('click', installTools);
}

async function loadTools(refresh) {
  if (state.toolsBusy) return;
  state.toolsBusy = true;
  renderTools();
  try {
    state.tools = await getJSON(`/api/tools${refresh ? '?refresh=1' : ''}`);
  } catch (e) {
    if (refresh) toast(explain(e), 'error');
  } finally {
    state.toolsBusy = false;
    state.setupSig = '';
    renderTools();
    if (state.view === 'overview' && state.status) renderSetup();
    if (state.view === 'settings') renderSettings();
  }
}

function renderTools() {
  const facts = $('#tools-facts');
  const tools = state.tools;
  $('#tools-check').disabled = state.toolsBusy;
  $('#tools-install').disabled = !tools?.downloader || isActive(state.status?.job);
  if (state.toolsBusy && !tools) {
    facts.replaceChildren(h('dt', { text: t('st.checking') }), h('dd'));
    return;
  }
  if (!tools) return;
  const missing = (text) => h('dd', { class: 'missing', text });
  const rows = [
    ['Python', tools.python_version ? h('dd', { text: `${tools.python_version} (${tools.python})` }) : missing(t('st.notFound'))],
    ['yt-dlp', tools.yt_dlp
      ? h('dd', { text: toolAge(tools.yt_dlp) ? `${tools.yt_dlp} (${t('st.released', { ago: ago(serverNow() - toolAge(tools.yt_dlp) * 86400) })})` : tools.yt_dlp })
      : missing(t('st.missing'))],
    ['gallery-dl', tools.gallery_dl ? h('dd', { text: tools.gallery_dl }) : missing(t('st.missing'))],
    ['ffmpeg', tools.ffmpeg ? h('dd', { text: tools.ffmpeg }) : missing(t('st.ffmpegMissing'))],
  ];
  if (tools.yt_dlp || tools.gallery_dl) {
    rows.push(['', h('dd', { class: 'help', text: t('st.found', { where: tools.venv ? (tools.venv_dir || t('st.venv')) : t('st.path') }) })]);
  }
  if (tools.error) rows.push(['', missing(tools.error)]);
  facts.replaceChildren(...rows.flatMap(([name, dd]) => [h('dt', { text: name }), dd]));
}

function renderAbout() {
  const s = state.status;
  const view = state.settings;
  if (!s) return;
  const link = h('a', { href: 'https://github.com/abel0x/x-link-collector', target: '_blank', rel: 'noopener noreferrer', text: 'github.com/abel0x/x-link-collector' });
  const rows = [
    [t('st.version'), h('dd', { text: s.version })],
    [t('st.linksFileNow'), h('dd', { class: 'mono', text: s.links_file })],
  ];
  if (view) {
    rows.push([t('st.configFile'), h('dd', {},
      h('span', { class: 'mono', text: view.config_file }), ' ',
      h('button', { type: 'button', class: 'btn small quiet', text: t('st.openFolder'), onclick: () => openThing('config') }))]);
    rows.push([t('st.historyFile'), h('dd', { class: 'mono', text: view.effective.history_file })]);
  }
  rows.push([t('st.source'), h('dd', {}, link)]);
  $('#about-facts').replaceChildren(...rows.flatMap(([name, dd]) => [h('dt', { text: name }), dd]));
}

/* ------------------------------------------------------------------ start */

function start() {
  const saved = remember('theme');
  if (saved === 'dark' || saved === 'light') document.documentElement.dataset.theme = saved;
  makeFormatters();
  applyStatic();

  $('#lang').addEventListener('click', switchLanguage);
  $('#theme').addEventListener('click', switchTheme);
  matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
    renderThemeButton();
    drawStrip();
  });
  window.addEventListener('hashchange', route);
  document.addEventListener('visibilitychange', () => {
    if (!document.hidden) kick();
  });
  window.addEventListener('beforeunload', (e) => {
    if (Object.keys(state.draft).length) e.preventDefault();
  });

  wireStrip();
  wireFloatingTips();
  wireLinks();
  wireDownload();
  wireFolder();
  wireSettings();
  wireViewer();
  wireGallery();

  route();
  loadSettings();
  loadTools(false);
}

start();
