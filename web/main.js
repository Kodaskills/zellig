'use strict';

// ── terminal ──────────────────────────────────────────────────────────────────

const SCENES = [
  {
    label: 'text', title: '~/projects/site',
    steps: [
      { type: 'cmd', text: 'zellig translate "Hello, world! How are you today?" --target fr' },
      { type: 'out', text: '» translating · backend: nllb-200-local · 0.18s', cls: 'term__c-muted', delay: 280 },
      { type: 'out', text: "Bonjour, le monde ! Comment allez-vous aujourd'hui ?", cls: 'term__c-ok', delay: 80 },
    ],
    pause: 1700,
  },
  {
    label: 'file', title: '~/docs',
    steps: [
      { type: 'cmd', text: 'zellig translate --input README.md --target ja --mode ollama' },
      { type: 'out', text: '» reading  README.md             (2.4 KB)',     cls: 'term__c-muted', delay: 200 },
      { type: 'out', text: '» preserving markdown structure (12 blocks)',   cls: 'term__c-muted', delay: 200 },
      { type: 'out', text: '» translating en → ja           via ollama/qwen2.5', cls: 'term__c-muted', delay: 220 },
      { type: 'out', text: '✓ written  README.ja.md         in 3.7s',       cls: 'term__c-ok',    delay: 320 },
    ],
    pause: 1800,
  },
  {
    label: 'directory', title: '~/app/locales',
    steps: [
      { type: 'cmd', text: 'zellig translate --dir ./locales --target es --target de --target it --mode deepl' },
      { type: 'out', text: '» scanning ./locales              42 files matched', cls: 'term__c-muted', delay: 240 },
      { type: 'out', text: '  ├─ en.po       → es de it     ✓ ✓ ✓',            cls: 'term__c-ok',    delay: 160 },
      { type: 'out', text: '  ├─ ui.json     → es de it     ✓ ✓ ✓',            cls: 'term__c-ok',    delay: 160 },
      { type: 'out', text: '  ├─ help.xliff  → es de it     ✓ ✓ ✓',            cls: 'term__c-ok',    delay: 160 },
      { type: 'out', text: '  └─ … 39 more   → es de it     ✓ ✓ ✓',            cls: 'term__c-ok',    delay: 200 },
      { type: 'out', text: '✓ 126 files written · 18,402 segments · 12.4s · $0.00 cached', cls: 'term__c-warn', delay: 320 },
    ],
    pause: 2200,
  },
  {
    label: 'interactive', title: '~ — zellig tui',
    steps: [
      { type: 'cmd', text: 'zellig tui' },
      // TUI paints as a full-screen block
      { type: 'out', text: ' ◆ zellig', cls: 'term__c-ok', delay: 220, sameLine: true },
      { type: 'out', text: '                                      Translate', cls: 'term__c-muted' },
      { type: 'out', text: '────────────────────────────────────────────────────────', cls: 'term__c-muted', delay: 0 },
      { type: 'out', text: ' Models │ Languages │ Config │ ', cls: 'term__c-muted', delay: 0, sameLine: true },
      { type: 'out', text: 'Translate', cls: 'term__c-ok', delay: 0, sameLine: true },
      { type: 'out', text: ' │ File │ Detect', cls: 'term__c-muted', delay: 0 },
      { type: 'out', text: '────────────────────────────────────────────────────────', cls: 'term__c-muted', delay: 0 },
      { type: 'out', text: '╭─ Source ↔ ─────────────────╮╭─ Target ───────────────╮', cls: 'term__c-muted', delay: 0 },
      { type: 'out', text: '│  English                   ││  French                │', delay: 0 },
      { type: 'out', text: '╰────────────────────────────╯╰────────────────────────╯', cls: 'term__c-muted', delay: 0 },
      { type: 'out', text: '╭─ Text ───────────────────────────────────────────────╮', cls: 'term__c-muted', delay: 0 },
      { type: 'out', text: '│  The early bird catches the worm.                    │', delay: 0 },
      { type: 'out', text: '╰──────────────────────────────────────────────────────╯', cls: 'term__c-muted', delay: 0 },
      // translation appears after Enter
      { type: 'out', text: '╭─ Translation ────────────────────────────────────────╮', cls: 'term__c-muted', delay: 480 },
      { type: 'out', text: '│  Le lève-tôt attrape le ver.                         │', cls: 'term__c-ok', delay: 0 },
      { type: 'out', text: '╰──────────────────────────────────────────────────────╯', cls: 'term__c-muted', delay: 0 },
      { type: 'out', text: '────────────────────────────────────────────────────────', cls: 'term__c-muted', delay: 0 },
      { type: 'out', text: '  Type  [Tab]=lang  [Enter]=translate  [Esc]=menu', cls: 'term__c-muted', delay: 0 },
    ],
    pause: 2400,
  },
];

let termGen = 0;
let termSceneIdx = 0;
const termSpeed = 1;

function termSleep(ms, gen) {
  return new Promise((resolve, reject) => {
    setTimeout(() => {
      if (termGen !== gen) reject();
      else resolve();
    }, Math.max(8, ms / termSpeed));
  });
}

async function runTerminal() {
  const body = document.getElementById('term-body');
  const titleEl = document.getElementById('term-title');
  const tabs = document.querySelectorAll('#term-tabs .term__tab');

  while (true) {
    const gen = ++termGen;
    const scene = SCENES[termSceneIdx];

    tabs.forEach((t, i) => t.classList.toggle('is-active', i === termSceneIdx));
    titleEl.textContent = scene.title + ' — zellig';
    body.innerHTML = '';

    const cursorLine = document.createElement('span');
    cursorLine.className = 'term__line';
    const cursor = document.createElement('span');
    cursor.className = 'term__cursor';
    cursorLine.appendChild(cursor);
    body.appendChild(cursorLine);

    let sameLineEl = null;

    function insertLine(text, cls) {
      const s = document.createElement('span');
      s.className = 'term__line' + (cls ? ' ' + cls : '');
      s.textContent = text;
      body.insertBefore(s, cursorLine);
      body.scrollTop = body.scrollHeight;
      return s;
    }

    try {
      for (const step of scene.steps) {
        if (termGen !== gen) return;

        if (step.type === 'cmd') {
          const line = document.createElement('span');
          line.className = 'term__line';
          const ps = document.createElement('span');
          ps.className = 'term__prompt';
          ps.textContent = '$ ';
          const cs = document.createElement('span');
          cs.className = 'term__c-cmd';
          line.appendChild(ps);
          line.appendChild(cs);
          body.insertBefore(line, cursorLine);

          cursorLine.removeChild(cursor);
          line.appendChild(cursor);

          await termSleep(120, gen);
          for (let c = 0; c < step.text.length; c++) {
            if (termGen !== gen) return;
            cs.textContent = step.text.slice(0, c + 1);
            await termSleep(18 + Math.random() * 22, gen);
          }
          await termSleep(280, gen);

          line.removeChild(cursor);
          cursorLine.appendChild(cursor);
          body.appendChild(cursorLine);
          sameLineEl = null;

        } else if (step.type === 'out') {
          await termSleep(step.delay || 120, gen);
          if (termGen !== gen) return;

          if (step.prompt) {
            const line = document.createElement('span');
            line.className = 'term__line';
            const ps = document.createElement('span');
            ps.className = 'term__c-warn';
            ps.textContent = '› ';
            const cs = document.createElement('span');
            cs.className = step.cls || '';
            cs.textContent = step.text;
            line.appendChild(ps);
            line.appendChild(cs);
            body.insertBefore(line, cursorLine);
            sameLineEl = null;
          } else if (step.sameLine) {
            if (!sameLineEl) {
              sameLineEl = document.createElement('span');
              sameLineEl.className = 'term__line';
              body.insertBefore(sameLineEl, cursorLine);
            }
            const s = document.createElement('span');
            s.className = step.cls || 'term__c-muted';
            s.textContent = step.text;
            sameLineEl.appendChild(s);
          } else {
            sameLineEl = null;
            insertLine(step.text, step.cls);
          }
          body.scrollTop = body.scrollHeight;
        }
      }

      await termSleep(scene.pause || 1500, gen);
      if (termGen !== gen) return;
      termSceneIdx = (termSceneIdx + 1) % SCENES.length;

    } catch (_) {
      // cancelled — outer while picks new scene
    }
  }
}

// ── tile band ─────────────────────────────────────────────────────────────────

function initTileBand() {
  const inner = document.getElementById('tile-band-inner');
  for (let i = 0; i < 28; i++) {
    const d = document.createElement('div');
    d.className = 'tile-band__cell';
    d.style.background = i % 4 === 0 ? 'var(--accent-2)' : i % 4 === 2 ? 'var(--accent)' : 'var(--line-2)';
    d.style.opacity = (i % 4 === 0 || i % 4 === 2) ? '0.55' : '0.4';
    inner.appendChild(d);
  }
}

// ── FAQ accordion ─────────────────────────────────────────────────────────────

function initFAQ() {
  document.querySelectorAll('.faq__item').forEach(item => {
    item.querySelector('.faq__q').addEventListener('click', () => {
      const isOpen = item.classList.contains('is-open');
      document.querySelectorAll('.faq__item').forEach(i => i.classList.remove('is-open'));
      if (!isOpen) item.classList.add('is-open');
    });
  });
}

// ── install copy button ───────────────────────────────────────────────────────

const SVG_COPY  = '<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5"><rect x="4.5" y="4.5" width="8" height="9" rx="1.5"/><path d="M9 1.5H3a1.5 1.5 0 0 0-1.5 1.5v7"/></svg>';
const SVG_CHECK = '<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 8.5l3.5 3.5L13 5"/></svg>';

function initInstall() {
  const btn = document.getElementById('install-copy-btn');
  btn.addEventListener('click', () => {
    const text = document.getElementById('install-cmd').textContent;
    navigator.clipboard?.writeText(text);
    btn.classList.add('is-copied');
    btn.innerHTML = SVG_CHECK + ' copied';
    setTimeout(() => {
      btn.classList.remove('is-copied');
      btn.innerHTML = SVG_COPY + ' copy';
    }, 1400);
  });
}

// ── theme ─────────────────────────────────────────────────────────────────────

function initTheme() {
  const html = document.documentElement;
  const btn = document.getElementById('theme-btn');
  const mq = window.matchMedia('(prefers-color-scheme: dark)');
  let mode = localStorage.getItem('theme') || 'system';

  function apply(m) {
    mode = m;
    btn.dataset.mode = m;
    const label = 'Theme: ' + m;
    btn.setAttribute('aria-label', label);
    btn.title = label;
    html.setAttribute('data-theme', m === 'system' ? (mq.matches ? 'dark' : 'light') : m);
    localStorage.setItem('theme', m);
  }

  mq.addEventListener('change', () => {
    if (mode === 'system') html.setAttribute('data-theme', mq.matches ? 'dark' : 'light');
  });

  btn.addEventListener('click', () => {
    btn.classList.remove('is-popping');
    void btn.offsetWidth;
    btn.classList.add('is-popping');
    btn.addEventListener('animationend', () => btn.classList.remove('is-popping'), { once: true });
    apply(mode === 'system' ? 'light' : mode === 'light' ? 'dark' : 'system');
  });

  apply(mode);
}

// ── tab clicks ────────────────────────────────────────────────────────────────

function initTermTabs() {
  document.querySelectorAll('#term-tabs .term__tab').forEach(btn => {
    btn.addEventListener('click', () => {
      termSceneIdx = parseInt(btn.dataset.scene, 10);
      termGen++;
    });
  });
}

// ── nav scrolled ─────────────────────────────────────────────────────────────

function initNavScroll() {
  const nav = document.querySelector('.nav');
  window.addEventListener('scroll', () => {
    nav.classList.toggle('is-scrolled', window.scrollY > 20);
  }, { passive: true });
}

// ── nav active section ────────────────────────────────────────────────────────

function initNavObserver() {
  const links = document.querySelectorAll('.nav__links a[href^="#"]');
  const sectionMap = Object.fromEntries(
    [...links].map(a => [a.getAttribute('href').slice(1), a])
  );
  const sections = Object.keys(sectionMap).map(id => document.getElementById(id)).filter(Boolean);

  const obs = new IntersectionObserver(entries => {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        Object.values(sectionMap).forEach(a => a.classList.remove('is-active'));
        const link = sectionMap[entry.target.id];
        if (link) link.classList.add('is-active');
      }
    });
  }, { rootMargin: '-30% 0px -60% 0px', threshold: 0 });

  sections.forEach(s => obs.observe(s));
}

// ── scroll reveal ─────────────────────────────────────────────────────────────

function initScrollReveal() {
  const groups = [
    '.section-head',
    '.features .feature',
    '.formats .fmt',
    '.backends .bcol',
    '.bench .bench__cell',
    '.timeline .tl-item',
    '.modes .mode',
    '.commu .commu__card',
    '.faq .faq__item',
  ];

  const obs = new IntersectionObserver(entries => {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        entry.target.classList.add('is-visible');
        obs.unobserve(entry.target);
      }
    });
  }, { rootMargin: '0px 0px -60px 0px', threshold: 0.08 });

  groups.forEach(sel => {
    document.querySelectorAll(sel).forEach((el, i) => {
      el.classList.add('reveal');
      el.style.setProperty('--delay', `${Math.min(i, 5) * 80}ms`);
      obs.observe(el);
    });
  });
}

// ── build date ────────────────────────────────────────────────────────────────

document.getElementById('footer-build').textContent =
  'build · ' + new Date().toISOString().slice(0, 10) + ' · sha 0d3a91f';

// ── init ──────────────────────────────────────────────────────────────────────

initTileBand();
initFAQ();
initInstall();
initTheme();
initTermTabs();
initNavScroll();
initNavObserver();
initScrollReveal();
runTerminal();
