(() => {
  'use strict';

  const $ = (id) => document.getElementById(id);
  const thread = $('thread');
  const messages = $('messages');
  const welcome = $('welcome');
  const prompt = $('prompt');
  const sendBtn = $('send-btn');
  const settings = $('settings');

  const chats = [];
  let current = null;
  let phase = null;
  let config = null;
  let sending = false;
  let toastTimer = null;
  let lastFocus = null;

  async function api(path, options = {}) {
    const response = await fetch('/api' + path, {
      ...options,
      headers: options.body ? { 'Content-Type': 'application/json' } : {},
    });
    let body = {};
    try {
      body = await response.json();
    } catch (_) {
      /* empty body */
    }
    if (!response.ok) throw new Error(body.error || `${response.status} ${response.statusText}`);
    return body;
  }

  function el(tag, className, text) {
    const node = document.createElement(tag);
    if (className) node.className = className;
    if (text != null) node.textContent = text;
    return node;
  }

  function icon(name) {
    const node = el('span', `ph ph-${name}`);
    node.setAttribute('aria-hidden', 'true');
    return node;
  }

  function toast(message, error = false) {
    const node = $('toast');
    node.textContent = message;
    node.classList.toggle('error', error);
    node.classList.remove('hidden');
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => node.classList.add('hidden'), 3800);
  }

  /* ── Daemon status ─────────────────────────────────────── */
  async function refreshStatus() {
    const row = $('daemon-row');
    const pill = $('status-pill');
    try {
      const reply = await api('/status');
      const online = reply.state === 'connected';
      row.classList.toggle('connected', online);
      row.classList.toggle('offline', !online);
      pill.classList.toggle('offline', !online);
      if (online) {
        const status = reply.status;
        phase = status.phase;
        $('daemon-text').textContent = `XBot ${status.version} · ${status.phase}`;
        pill.textContent = status.monitoring ? 'Watching for problems' : 'Not watching yet';
      } else {
        $('daemon-text').textContent = "XBot isn't running";
        pill.textContent = "XBot isn't running";
      }
    } catch (error) {
      row.classList.remove('connected');
      row.classList.add('offline');
      pill.classList.add('offline');
      $('daemon-text').textContent = 'Could not reach XBot';
      pill.textContent = 'Could not reach XBot';
    }
  }

  async function refreshIncidents() {
    const list = $('incident-list');
    let reply;
    try {
      reply = await api('/incidents');
    } catch (error) {
      reply = { state: 'error', message: error.message };
    }
    list.replaceChildren();
    if (reply.state === 'available' && Array.isArray(reply.incidents) && reply.incidents.length) {
      for (const incident of reply.incidents) {
        const item = el('div', 'side-item');
        item.setAttribute('role', 'listitem');
        item.append(
          icon(incident.severity === 'info' ? 'info' : 'warning-circle'),
          el('span', 'side-item-text', incident.title),
        );
        list.append(item);
      }
      return;
    }
    const text = {
      available: 'No problems found.',
      unavailable: "Problem detection isn't on in this version yet.",
      offline: 'Shown when XBot is running.',
    }[reply.state] || 'Could not load problems.';
    list.append(el('div', 'side-empty', text));
  }

  function refreshAll() {
    refreshStatus();
    refreshIncidents();
  }

  /* ── Chats ─────────────────────────────────────────────── */
  function renderChatList() {
    const list = $('chat-list');
    list.replaceChildren();
    if (!chats.length) {
      list.append(el('div', 'side-empty', 'Your chats appear here.'));
      return;
    }
    for (const chat of chats) {
      const item = el('button', 'side-item pressable' + (chat === current ? ' active' : ''));
      item.type = 'button';
      item.setAttribute('role', 'listitem');
      if (chat === current) item.setAttribute('aria-current', 'true');
      item.append(icon('chat-teardrop-text'), el('span', 'side-item-text', chat.title));
      item.addEventListener('click', () => openChat(chat));
      list.append(item);
    }
  }

  function renderEntry(entry) {
    if (entry.kind === 'message') return el('div', `message ${entry.role}`, entry.text);
    const notice = el('div', 'notice' + (entry.error ? ' error' : ''));
    notice.setAttribute('role', entry.error ? 'alert' : 'note');
    const body = el('div');
    body.append(el('div', 'notice-title', entry.title), el('div', 'notice-body', entry.text));
    notice.append(icon(entry.error ? 'warning-circle' : 'info'), body);
    return notice;
  }

  function renderThread() {
    const entries = current ? current.entries : [];
    welcome.classList.toggle('hidden', entries.length > 0);
    messages.replaceChildren(...entries.map(renderEntry));
    $('chat-title').textContent = current ? current.title : 'New Chat';
    thread.scrollTop = thread.scrollHeight;
  }

  function openChat(chat) {
    current = chat;
    renderChatList();
    renderThread();
    prompt.focus();
  }

  function newChat() {
    current = null;
    renderChatList();
    renderThread();
    prompt.value = '';
    resizePrompt();
    prompt.focus();
  }

  function add(entry) {
    current.entries.push(entry);
    const node = renderEntry(entry);
    messages.append(node);
    welcome.classList.add('hidden');
    node.scrollIntoView({ block: 'end' });
  }

  function replyNotice(reply) {
    if (reply.state === 'sent') {
      return { kind: 'notice', title: 'Sent to XBot', text: 'XBot is looking into it.' };
    }
    if (reply.state === 'unavailable') {
      const version = phase ? ` (${phase})` : '';
      return {
        kind: 'notice',
        title: "XBot can't chat yet",
        text: `This version of XBot${version} only reports its status. Chat arrives in a later update.`,
      };
    }
    if (reply.state === 'offline') {
      return {
        kind: 'notice',
        error: true,
        title: "XBot isn't running",
        text: 'XBot starts with your session. Try again in a moment.',
      };
    }
    return { kind: 'notice', error: true, title: 'Something went wrong', text: reply.message || '' };
  }

  async function send(text) {
    text = text.trim();
    if (!text || sending) return;
    if (!current) {
      current = { title: text.length > 48 ? text.slice(0, 47) + '…' : text, entries: [] };
      chats.unshift(current);
      renderChatList();
      $('chat-title').textContent = current.title;
    }
    const chat = current;
    add({ kind: 'message', role: 'user', text });
    prompt.value = '';
    resizePrompt();
    setSending(true);
    let reply;
    try {
      reply = await api('/chat', { method: 'POST', body: JSON.stringify({ text }) });
    } catch (error) {
      reply = { state: 'error', message: error.message };
    }
    setSending(false);
    const notice = replyNotice(reply);
    if (current === chat) add(notice);
    else chat.entries.push(notice);
    if (reply.state === 'offline') refreshStatus();
  }

  function setSending(value) {
    sending = value;
    sendBtn.disabled = value || !prompt.value.trim();
  }

  function resizePrompt() {
    prompt.style.height = 'auto';
    prompt.style.height = Math.min(prompt.scrollHeight, 180) + 'px';
    sendBtn.disabled = sending || !prompt.value.trim();
  }

  /* ── Settings ──────────────────────────────────────────── */
  const switches = () => [...settings.querySelectorAll('.switch')];

  function readKey(object, key) {
    return key.split('.').reduce((value, part) => (value ? value[part] : undefined), object);
  }

  function writeKey(object, key, value) {
    const parts = key.split('.');
    const last = parts.pop();
    const target = parts.reduce((value, part) => (value[part] = value[part] || {}), object);
    target[last] = value;
  }

  async function openSettings() {
    lastFocus = document.activeElement;
    $('settings-error').classList.add('hidden');
    try {
      config = (await api('/config')).config;
    } catch (error) {
      toast(`Could not load settings: ${error.message}`, true);
      return;
    }
    for (const toggle of switches()) {
      toggle.setAttribute('aria-checked', String(Boolean(readKey(config, toggle.dataset.key))));
    }
    settings.classList.remove('hidden');
    const first = switches().find((toggle) => !toggle.disabled);
    if (first) first.focus();
  }

  function closeSettings() {
    settings.classList.add('hidden');
    if (lastFocus && lastFocus.focus) lastFocus.focus();
  }

  async function saveSettings(event) {
    event.preventDefault();
    const next = JSON.parse(JSON.stringify(config));
    for (const toggle of switches()) {
      if (!toggle.disabled) writeKey(next, toggle.dataset.key, toggle.getAttribute('aria-checked') === 'true');
    }
    try {
      config = (await api('/config', { method: 'PUT', body: JSON.stringify(next) })).config;
      closeSettings();
      toast('Settings saved');
    } catch (error) {
      const message = $('settings-error');
      message.textContent = `Could not save settings: ${error.message}`;
      message.classList.remove('hidden');
    }
  }

  /* ── Theme ─────────────────────────────────────────────── */
  const themeToggle = $('theme-toggle');

  function applyTheme(theme) {
    document.documentElement.setAttribute('data-theme', theme);
    const meta = $('meta-theme-color');
    if (meta) meta.content = theme === 'dark' ? '#0a0a0b' : '#ffffff';
    themeToggle.title = theme === 'dark' ? 'Switch to light mode' : 'Switch to dark mode';
    themeToggle.setAttribute('aria-label', themeToggle.title);
  }

  themeToggle.addEventListener('click', () => {
    const next = document.documentElement.getAttribute('data-theme') === 'dark' ? 'light' : 'dark';
    applyTheme(next);
    try {
      localStorage.setItem('xbot-theme', next);
    } catch (_) {
      /* ignore */
    }
  });

  // Follow the system colour scheme until the user picks a theme.
  try {
    window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (event) => {
      if (!localStorage.getItem('xbot-theme')) applyTheme(event.matches ? 'dark' : 'light');
    });
  } catch (_) {
    /* ignore */
  }

  /* ── Events ────────────────────────────────────────────── */
  $('composer').addEventListener('submit', (event) => {
    event.preventDefault();
    send(prompt.value);
  });
  prompt.addEventListener('input', resizePrompt);
  prompt.addEventListener('keydown', (event) => {
    if (event.key === 'Enter' && !event.shiftKey && !event.isComposing) {
      event.preventDefault();
      send(prompt.value);
    }
  });
  for (const suggestion of document.querySelectorAll('.suggestion')) {
    suggestion.addEventListener('click', () => send(suggestion.dataset.prompt));
  }
  $('new-chat').addEventListener('click', newChat);
  $('refresh-btn').addEventListener('click', refreshAll);
  $('settings-btn').addEventListener('click', openSettings);
  $('settings-form').addEventListener('submit', saveSettings);
  for (const closer of settings.querySelectorAll('[data-close]')) closer.addEventListener('click', closeSettings);
  for (const toggle of switches()) {
    toggle.addEventListener('click', () => {
      toggle.setAttribute('aria-checked', String(toggle.getAttribute('aria-checked') !== 'true'));
    });
  }
  window.addEventListener('keydown', (event) => {
    const mod = event.ctrlKey || event.metaKey;
    if (event.key === 'Escape' && !settings.classList.contains('hidden')) {
      event.preventDefault();
      closeSettings();
    } else if (mod && event.key.toLowerCase() === 'n') {
      event.preventDefault();
      newChat();
    } else if (mod && event.key === ',') {
      event.preventDefault();
      openSettings();
    }
  });
  // Refresh on return rather than polling, so an idle window does no work.
  window.addEventListener('focus', refreshAll);

  applyTheme(document.documentElement.getAttribute('data-theme') === 'dark' ? 'dark' : 'light');
  renderChatList();
  resizePrompt();
  refreshAll();
  prompt.focus();
})();
