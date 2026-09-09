/**
 * Proves that long pages can be scrolled.
 *
 * A flex or grid item defaults to min-height: auto, meaning it refuses to become
 * shorter than its own content. .content therefore grew to fit everything
 * instead of being capped at the space below the top bar, overflow-y: auto had
 * nothing to overflow, and body's overflow: hidden clipped the rest. The page
 * looked finished and simply would not scroll.
 *
 * That is pure layout, so reasoning about it is not proof. This loads the real
 * stylesheet into a real browser, puts more content on the page than fits, and
 * checks the box can actually be scrolled. Against the old CSS it fails with
 * clientHeight 2415px in an 800px window and scrollTop stuck at 0.
 */
import { readFileSync, existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, '..');

// Playwright is not a dependency of this project — a browser download is a lot
// to add to every install for one test. When it is not there this skips, the
// same way the ffmpeg tests do.
let chromium;
try {
  ({ chromium } = createRequire(import.meta.url)('playwright'));
} catch {
  console.log('  skipped: playwright is not installed (npm i -g playwright to run this)');
  process.exit(0);
}

const css = readFileSync(join(root, 'apps/desktop/src/styles.css'), 'utf8');

// Whichever chromium is on this machine.
const CANDIDATES = [
  '/opt/pw-browsers/chromium-1194/chrome-linux/chrome',
  '/opt/pw-browsers/chromium/chrome-linux/chrome',
];
const executablePath = CANDIDATES.find((p) => existsSync(p));

// The app's real shell: sidebar + main + topbar + content, as App.tsx renders it.
const html = `<!doctype html><html><head><meta charset="utf-8"><style>${css}</style></head>
<body><div id="root"><div class="app">
  <aside class="sidebar"><div class="brand">GameHub</div>
    ${Array.from({ length: 12 }, (_, i) => `<button class="nav-item">Item ${i}</button>`).join('')}
  </aside>
  <div class="main">
    <div class="topbar"><input class="search" placeholder="Search games…"><button class="btn">Rescan</button></div>
    <div class="content" id="content">
      ${Array.from({ length: 60 }, (_, i) => `<p style="margin:0 0 18px">Row ${i} — content that runs well past the bottom of the window.</p>`).join('')}
      <p id="last">THE LAST LINE</p>
    </div>
  </div>
</div></div></body></html>`;

const browser = await chromium.launch(executablePath ? { executablePath } : {});
const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
await page.setContent(html);

let failures = 0;
const check = (name, condition, detail = '') => {
  if (condition) console.log(`  ok  ${name}`);
  else { failures++; console.error(`  FAIL  ${name}${detail ? `\n        ${detail}` : ''}`); }
};

const box = await page.evaluate(() => {
  const el = document.getElementById('content');
  return {
    client: el.clientHeight,
    scroll: el.scrollHeight,
    windowHeight: window.innerHeight,
  };
});

check(
  'the content box is capped at the window, not grown to fit',
  box.client < box.windowHeight,
  `client=${box.client}px, window=${box.windowHeight}px — if these are equal or client is larger, min-height:0 is missing`,
);

check(
  'there is genuinely more content than fits',
  box.scroll > box.client,
  `scrollHeight=${box.scroll}, clientHeight=${box.client}`,
);

// The real question: can it actually be scrolled?
const scrolled = await page.evaluate(() => {
  const el = document.getElementById('content');
  el.scrollTop = 99999;
  return el.scrollTop;
});
check('the content actually scrolls', scrolled > 0, `scrollTop stayed at ${scrolled}`);

// And is the bottom of the page reachable on screen?
const lastVisible = await page.evaluate(() => {
  const last = document.getElementById('last');
  last.scrollIntoView();
  const rect = last.getBoundingClientRect();
  return rect.top >= 0 && rect.bottom <= window.innerHeight + 1;
});
check('the last line can be brought on screen', lastVisible);

// The sidebar must scroll too rather than running off the bottom.
const sidebar = await page.evaluate(() => {
  const el = document.querySelector('.sidebar');
  return { client: el.clientHeight, window: window.innerHeight };
});
check(
  'the sidebar is capped at the window too',
  sidebar.client <= sidebar.window,
  `sidebar=${sidebar.client}px, window=${sidebar.window}px`,
);

// Nothing may spill past the window and get clipped by body overflow:hidden.
const spill = await page.evaluate(() => document.documentElement.scrollHeight - window.innerHeight);
check('nothing spills past the window', spill <= 0, `overflowing by ${spill}px`);

await browser.close();
console.log(failures === 0 ? '\nscroll behaves correctly\n' : `\n${failures} failed\n`);
process.exit(failures === 0 ? 0 : 1);
