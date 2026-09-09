/**
 * Tests for the in-app shortcut matching rules.
 *
 * The logic lives in TypeScript because the app is TypeScript, so this
 * transpiles that one file with the compiler the project already depends on.
 * No test framework and no new dependency — the same shape as the other
 * script tests, which run identically on a clean CI runner.
 */
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, mkdtempSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, '..');
const require = createRequire(join(root, 'apps/desktop/package.json'));
const ts = require('typescript');

const source = readFileSync(join(root, 'apps/desktop/src/shortcutMatch.ts'), 'utf8');
const { outputText } = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
});

const scratch = mkdtempSync(join(tmpdir(), 'gamehub-shortcut-'));
const compiled = join(scratch, 'shortcutMatch.mjs');
writeFileSync(compiled, outputText);
const { acceleratorFor, isTypingTarget, matchAction } = await import(`file://${compiled}`);

let failures = 0;
function test(name, fn) {
  try {
    fn();
    console.log(`  ok  ${name}`);
  } catch (error) {
    failures += 1;
    console.error(`  FAIL  ${name}\n        ${error.message}`);
  }
}

const key = (k, mods = {}) => ({
  key: k,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  metaKey: false,
  ...mods,
});

// The registry as the backend reports it, global entries included on purpose.
const BINDINGS = [
  { action: 'screenshot', scope: 'global', binding: 'F9' },
  { action: 'search', scope: 'app', binding: 'Ctrl+F' },
  { action: 'library', scope: 'app', binding: 'Ctrl+1' },
  { action: 'screenshots', scope: 'app', binding: 'Ctrl+4' },
  { action: 'rescan', scope: 'app', binding: 'F5' },
];

test('a plain key becomes its own accelerator', () => {
  assert.equal(acceleratorFor(key('F5')), 'F5');
});

test('modifiers come out in a fixed order', () => {
  assert.equal(
    acceleratorFor(key('f', { ctrlKey: true, shiftKey: true })),
    'Ctrl+Shift+F',
  );
});

test('a letter is upper-cased so case never decides a match', () => {
  assert.equal(acceleratorFor(key('f', { ctrlKey: true })), 'Ctrl+F');
});

test('space has a name rather than being a blank', () => {
  assert.equal(acceleratorFor(key(' ', { ctrlKey: true })), 'Ctrl+Space');
});

test('a modifier on its own is not a shortcut yet', () => {
  assert.equal(acceleratorFor(key('Control', { ctrlKey: true })), '');
  assert.equal(acceleratorFor(key('Shift', { shiftKey: true })), '');
});

test('an app shortcut matches', () => {
  assert.equal(matchAction('Ctrl+F', BINDINGS, false), 'search');
  assert.equal(matchAction('Ctrl+4', BINDINGS, false), 'screenshots');
});

test('matching ignores case', () => {
  assert.equal(matchAction('ctrl+f', BINDINGS, false), 'search');
});

test('a global shortcut is NOT handled here', () => {
  // Windows already delivers it through the backend. Handling it again would
  // take two screenshots every time GameHub had focus.
  assert.equal(matchAction('F9', BINDINGS, false), null);
});

test('an unbound key does nothing', () => {
  assert.equal(matchAction('Ctrl+9', BINDINGS, false), null);
});

test('an empty accelerator does nothing', () => {
  assert.equal(matchAction('', BINDINGS, false), null);
});

test('a bare key does not fire while typing', () => {
  // Otherwise F5 could never be typed, and a bare-letter binding would make
  // the search box unusable.
  assert.equal(matchAction('F5', BINDINGS, true), null);
});

test('a shortcut with a modifier still fires while typing', () => {
  assert.equal(matchAction('Ctrl+F', BINDINGS, true), 'search');
});

test('a bare key fires when not typing', () => {
  assert.equal(matchAction('F5', BINDINGS, false), 'rescan');
});

test('text fields and editable areas count as typing', () => {
  assert.equal(isTypingTarget('INPUT', false), true);
  assert.equal(isTypingTarget('TEXTAREA', false), true);
  assert.equal(isTypingTarget('SELECT', false), true);
  assert.equal(isTypingTarget('DIV', true), true);
});

test('ordinary elements do not count as typing', () => {
  assert.equal(isTypingTarget('DIV', false), false);
  assert.equal(isTypingTarget('BUTTON', false), false);
  assert.equal(isTypingTarget(undefined, false), false);
});

// --- the registry and the app must agree ------------------------------------
//
// The Rust registry decides which shortcuts exist; App.tsx decides what each one
// does. A navigation action in one and not the other is a shortcut that appears
// in Settings and does nothing, which is exactly the bug this round fixed.
const rustSource = readFileSync(join(root, 'packages/game-detection/src/shortcuts.rs'), 'utf8');
const appSource = readFileSync(join(root, 'apps/desktop/src/App.tsx'), 'utf8');

const idBlock = rustSource.slice(rustSource.indexOf('pub fn id('), rustSource.indexOf('pub fn label('));
const ids = [...idBlock.matchAll(/=>\s*"([a-z_]+)"/g)].map((m) => m[1]);

// Actions the backend handles itself, so App.tsx never sees them.
const BACKEND_ONLY = new Set([
  'open_gamehub',
  'quick_tools',
  'screenshot',
  'save_replay',
  'toggle_overlay',
  'toggle_replay',
  // Freezing needs the process list, which only the backend has.
  'freeze_game',
]);

test('every action id is handled somewhere', () => {
  assert.ok(ids.length > 10, `expected a full registry, found ${ids.length}`);
  const missing = ids
    .filter((id) => !BACKEND_ONLY.has(id))
    .filter((id) => !appSource.includes(`case '${id}'`));
  assert.deepEqual(missing, [], `App.tsx has no case for: ${missing.join(', ')}`);
});

test('the shortcut hook is actually mounted', () => {
  // The handler existing but never being called is the failure mode that made
  // half the shortcuts decorative.
  assert.ok(appSource.includes('useAppShortcuts('), 'App.tsx must call useAppShortcuts');
});

console.log(failures === 0 ? '\nall shortcut tests passed' : `\n${failures} failed`);
process.exit(failures === 0 ? 0 : 1);
