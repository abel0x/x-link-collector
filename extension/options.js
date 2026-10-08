// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 abel0x <https://github.com/abel0x>

'use strict';

/*
 * The extension's options page. It is also where the toolbar icon leads when
 * the receiver does not answer, so it says first whether it does, and how to
 * start it when not. While the receiver is down the page keeps checking, and
 * turns into "connected" by itself once it is started.
 */

const DEFAULT_PORT = 9876;
const RECHECK_MS = 3000;

const WORDS = {
  en: {
    checking: 'Checking…',
    up: 'Connected to x-link-receiver on port {port}.',
    upDetail: ['{n} link collected so far.', '{n} links collected so far.'],
    down: 'x-link-receiver is not answering on port {port}.',
    downDetail: 'Tweets you open stay in their tabs until it answers, so nothing is lost.',
    panel: 'Open the panel',
    retry: 'Check again',
    startTitle: 'Start x-link-receiver',
    startWindows: 'Double-click <code>x-link-receiver.exe</code>. The panel opens in your browser.',
    startUnix: 'Run <code>x-link-receiver</code>, or <code>make run</code> in the project folder. <code>make service</code> starts it at every login.',
    portLabel: 'Receiver port',
    portHelp: 'Change it only if x-link-receiver runs on another port. Its panel shows the port under Settings.',
    save: 'Save',
    saved: 'Saved.',
    invalid: 'A port is a whole number from 1 to 65535.',
  },
  tr: {
    checking: 'Kontrol ediliyor…',
    up: 'x-link-receiver {port} portunda çalışıyor.',
    upDetail: ['Şu ana kadar {n} bağlantı toplandı.', 'Şu ana kadar {n} bağlantı toplandı.'],
    down: 'x-link-receiver {port} portunda yanıt vermiyor.',
    downDetail: 'Program yanıt verene kadar açtığınız tweet’ler sekmelerinde kalır; hiçbir bağlantı kaybolmaz.',
    panel: 'Paneli aç',
    retry: 'Yeniden dene',
    startTitle: 'x-link-receiver programını başlatın',
    startWindows: '<code>x-link-receiver.exe</code> dosyasına çift tıklayın. Panel tarayıcınızda açılır.',
    startUnix: '<code>x-link-receiver</code> programını çalıştırın ya da proje klasöründe <code>make run</code> komutunu verin. <code>make service</code> komutu programı her oturum açılışında başlatır.',
    portLabel: 'Alıcı portu',
    portHelp: 'Yalnızca x-link-receiver başka bir portta çalışıyorsa değiştirin. Kullanılan portu panelin Ayarlar sayfasında görebilirsiniz.',
    save: 'Kaydet',
    saved: 'Kaydedildi.',
    invalid: 'Port, 1 ile 65535 arasında bir tam sayı olmalı.',
  },
};

const ui = (chrome.i18n?.getUILanguage?.() || navigator.language || 'en').toLowerCase();
const lang = ui.startsWith('tr') ? 'tr' : 'en';
const $ = (id) => document.getElementById(id);

function t(key, vars = {}) {
  let text = WORDS[lang][key];
  if (Array.isArray(text)) text = vars.n === 1 ? text[0] : text[1];
  return text.replace(/\{(\w+)\}/g, (whole, name) => (name in vars ? String(vars[name]) : whole));
}

/** Text from WORDS that holds nothing but <code> runs, built without innerHTML. */
function withCode(node, text) {
  node.replaceChildren(...text.split(/(<code>.*?<\/code>)/).filter(Boolean).map((part) => {
    const code = /^<code>(.*)<\/code>$/.exec(part);
    if (!code) return document.createTextNode(part);
    const element = document.createElement('code');
    element.textContent = code[1];
    return element;
  }));
}

let port = DEFAULT_PORT;
let recheck = 0;

async function check() {
  clearTimeout(recheck);
  const status = $('state');
  status.className = 'state';
  $('state-text').textContent = t('checking');

  let health = null;
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), 2500);
  try {
    const response = await fetch(`http://127.0.0.1:${port}/health`, {
      headers: { 'X-Link-Collector': 'extension' },
      cache: 'no-store',
      signal: controller.signal,
    });
    if (response.ok) health = await response.json();
  } catch {
    // not running, or not on this port
  } finally {
    clearTimeout(timer);
  }

  const up = Boolean(health);
  status.className = up ? 'state up' : 'state down';
  $('state-text').textContent = t(up ? 'up' : 'down', { port });
  $('detail').textContent = up ? t('upDetail', { n: health.total ?? 0 }) : t('downDetail');
  $('panel').hidden = !up;
  $('start').hidden = up;
  if (!up && !document.hidden) recheck = setTimeout(check, RECHECK_MS);
}

async function save(event) {
  event.preventDefault();
  const input = $('port');
  const value = Number(input.value);
  const valid = Number.isInteger(value) && value >= 1 && value <= 65535;
  input.setAttribute('aria-invalid', String(!valid));
  if (!valid) {
    $('saved').textContent = t('invalid');
    return;
  }
  await chrome.storage.local.set({ port: value });
  port = value;
  $('saved').textContent = t('saved');
  check();
}

async function start() {
  document.documentElement.lang = lang;
  $('panel').textContent = t('panel');
  $('retry').textContent = t('retry');
  $('start-title').textContent = t('startTitle');
  withCode($('start-windows'), t('startWindows'));
  withCode($('start-unix'), t('startUnix'));
  $('port-label').textContent = t('portLabel');
  $('port-help').textContent = t('portHelp');
  $('save').textContent = t('save');

  try {
    const stored = await chrome.storage.local.get({ port: DEFAULT_PORT });
    const n = Number(stored.port);
    if (Number.isInteger(n) && n >= 1 && n <= 65535) port = n;
  } catch {
    // the default stands
  }
  $('port').value = port;

  $('panel').addEventListener('click', () => chrome.tabs.create({ url: `http://127.0.0.1:${port}/` }));
  $('retry').addEventListener('click', check);
  $('form').addEventListener('submit', save);
  $('port').addEventListener('input', () => {
    $('saved').textContent = '';
  });
  document.addEventListener('visibilitychange', () => {
    if (!document.hidden) check();
  });
  check();
}

start();
