/**
 * The language guard: when the language changes, everything changes.
 *
 * Four things are checked, and each one is a way a word could be left behind:
 *
 *  1. Every language file has exactly the keys of en.json, with the same
 *     `{placeholders}` — so no screen can fall back to English for one label.
 *  2. No .tsx file contains literal text in JSX (or in a placeholder, title,
 *     alt or aria-label attribute) that is not going through t().
 *  3. Every message code the Rust side sends (`msg::plain("x")`,
 *     `msg::code("x", …)`, `"@x…"`) has a `be.x` entry, and the Rust side
 *     contains no sentence literals of its own.
 *  4. The tray menu (the one thing Rust draws) knows the same languages as
 *     the web view.
 *
 * Runs with node alone; no framework.
 */
import assert from 'node:assert/strict';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, dirname, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, '..');
const desktop = join(root, 'apps/desktop');
const require = createRequire(join(desktop, 'package.json'));
const ts = require('typescript');

const i18nDir = join(desktop, 'src/i18n');
const en = JSON.parse(readFileSync(join(i18nDir, 'en.json'), 'utf8'));
const enKeys = Object.keys(en);

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

function walk(dir, ext, out = []) {
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) {
      if (name !== 'node_modules' && name !== 'target' && name !== 'dist') walk(path, ext, out);
    } else if (path.endsWith(ext)) {
      out.push(path);
    }
  }
  return out;
}

const placeholders = (s) => (s.match(/\{[^}]+\}/g) ?? []).sort().join(',');

// ---------------------------------------------------------------- languages
const indexSource = readFileSync(join(i18nDir, 'index.ts'), 'utf8');
const languageCodes = [...indexSource.matchAll(/code: '([a-z]{2})'/g)].map((m) => m[1]);

test('the web view offers ten languages, English first', () => {
  assert.equal(languageCodes[0], 'en');
  assert.equal(new Set(languageCodes).size, 10, languageCodes.join(','));
  assert.ok(languageCodes.includes('nb'), 'Norwegian is one of them');
});

test('English is the default until a language is chosen', () => {
  assert.match(indexSource, /DEFAULT_LANGUAGE = 'en'/);
  const store = readFileSync(join(desktop, 'src-tauri/src/store.rs'), 'utf8');
  assert.match(store, /language: "en"\.into\(\)/, 'store.rs defaults the language to en');
  assert.match(store, /language_chosen: false/, 'store.rs defaults language_chosen to false');
});

for (const code of languageCodes) {
  const dictionary = JSON.parse(readFileSync(join(i18nDir, `${code}.json`), 'utf8'));
  test(`${code}.json has every key of en.json and nothing else`, () => {
    const keys = Object.keys(dictionary);
    const missing = enKeys.filter((k) => !(k in dictionary));
    const extra = keys.filter((k) => !(k in en));
    assert.deepEqual(missing, [], `missing: ${missing.slice(0, 5).join(', ')}`);
    assert.deepEqual(extra, [], `extra: ${extra.slice(0, 5).join(', ')}`);
  });
  test(`${code}.json keeps every placeholder and has no empty text`, () => {
    for (const key of enKeys) {
      assert.equal(typeof dictionary[key], 'string', `${key} is a string`);
      assert.ok(dictionary[key].trim().length > 0 || en[key].trim().length === 0, `${key} is not empty`);
      assert.equal(placeholders(dictionary[key]), placeholders(en[key]), `${key} placeholders`);
    }
  });
  if (code !== 'en') {
    test(`${code}.json is a translation, not a copy of English`, () => {
      const wordy = enKeys.filter((k) => /[a-z]{4,}/i.test(en[k]) && en[k].split(' ').length >= 3);
      const same = wordy.filter((k) => dictionary[k] === en[k]);
      assert.ok(
        same.length <= wordy.length * 0.02,
        `${same.length} of ${wordy.length} longer texts are identical to English: ${same.slice(0, 5).join(', ')}`,
      );
    });
  }
}

// ---------------------------------------------------------------- TSX text
const ATTRIBUTES = new Set(['placeholder', 'title', 'alt', 'aria-label', 'label', 'blurb', 'body', 'confirmLabel', 'hint']);
// Things that are the same in every language: the product name, key names,
// units, and the on-disk folder prefix a freeze point is stored under.
const SAME_EVERYWHERE = /^(GameHub|Ctrl\+\w+|fps|% CPU ·|\\…\\Frys_…)$/;
const hasLetters = (s) => /[A-Za-zÆØÅæøå]{2,}/.test(s) && !SAME_EVERYWHERE.test(s.trim());

function literalTextIn(file) {
  const source = readFileSync(file, 'utf8');
  const sf = ts.createSourceFile(file, source, ts.ScriptTarget.ES2022, true, ts.ScriptKind.TSX);
  const found = [];
  const report = (node, text) => {
    const { line } = sf.getLineAndCharacterOfPosition(node.getStart());
    found.push(`${relative(root, file)}:${line + 1} "${text.trim().slice(0, 40)}"`);
  };
  const visit = (node) => {
    if (ts.isJsxText(node)) {
      if (hasLetters(node.text)) report(node, node.text);
    } else if (ts.isJsxAttribute(node) && node.initializer) {
      const name = node.name.getText();
      if (ATTRIBUTES.has(name)) {
        const init = node.initializer;
        if (ts.isStringLiteral(init) && hasLetters(init.text)) report(node, init.text);
        if (ts.isJsxExpression(init) && init.expression && ts.isStringLiteral(init.expression) && hasLetters(init.expression.text)) {
          report(node, init.expression.text);
        }
      }
    } else if (ts.isJsxExpression(node) && node.expression) {
      // {'Some words'} and {`Some words`} straight inside JSX.
      const e = node.expression;
      if ((ts.isStringLiteral(e) || ts.isNoSubstitutionTemplateLiteral(e)) && hasLetters(e.text) && ts.isJsxElement(node.parent)) {
        report(node, e.text);
      }
    }
    ts.forEachChild(node, visit);
  };
  visit(sf);
  return found;
}

test('no .tsx file shows literal text outside t()', () => {
  const files = walk(join(desktop, 'src'), '.tsx');
  assert.ok(files.length > 20, `found ${files.length} tsx files`);
  const found = files.flatMap(literalTextIn);
  assert.deepEqual(found, [], `literal text:\n        ${found.join('\n        ')}`);
});

test('the language chooser exists and preselects English', () => {
  const chooser = readFileSync(join(desktop, 'src/views/LanguageChooser.tsx'), 'utf8');
  assert.ok(chooser.length > 200);
  const app = readFileSync(join(desktop, 'src/App.tsx'), 'utf8');
  assert.match(app, /LanguageChooser/);
  assert.match(app, /initial="en"/, 'English is preselected');
  assert.match(app, /languageChosen/, 'the chooser is shown until a language is chosen');
});

// ---------------------------------------------------------------- Rust
const rustFiles = [
  ...walk(join(desktop, 'src-tauri/src'), '.rs'),
  ...walk(join(root, 'packages/game-detection/src'), '.rs'),
  ...walk(join(root, 'packages/launcher-adapters/src'), '.rs'),
];

function stripTests(source) {
  const at = source.indexOf('#[cfg(test)]');
  return at === -1 ? source : source.slice(0, at);
}

test('every message code the Rust side sends has a be.* text', () => {
  const missing = new Set();
  for (const file of rustFiles) {
    const source = stripTests(readFileSync(file, 'utf8'));
    for (const m of source.matchAll(/msg::(?:plain|code)\(\s*"([a-z_]+)"/g)) {
      if (!(`be.${m[1]}` in en)) missing.add(`${relative(root, file)} → ${m[1]}`);
    }
    for (const m of source.matchAll(/msg::plain\(if [^{]+\{ "([a-z_]+)" \} else \{ "([a-z_]+)" \}/g)) {
      for (const key of [m[1], m[2]]) if (!(`be.${key}` in en)) missing.add(`${relative(root, file)} → ${key}`);
    }
    for (const m of source.matchAll(/"@([a-z_]+)[|"]/g)) {
      if (!(`be.${m[1]}` in en)) missing.add(`${relative(root, file)} → ${m[1]}`);
    }
  }
  assert.deepEqual([...missing], []);
});

test('the Rust side has no sentences of its own for the user', () => {
  const found = [];
  const sentence = /"([A-ZÆØÅ][a-zæøå]+(?: [^"]+){2,}[.:!?])"/g;
  for (const file of rustFiles) {
    const lines = stripTests(readFileSync(file, 'utf8')).split('\n');
    lines.forEach((line, index) => {
      const trimmed = line.trim();
      if (trimmed.startsWith('//') || /eprintln!|println!|expect\(|panic!|assert|cargo:warning|with_limitation|limitation/.test(line)) return;
      for (const m of line.matchAll(sentence)) found.push(`${relative(root, file)}:${index + 1} ${m[1].slice(0, 50)}`);
    });
  }
  assert.deepEqual(found, [], `sentences:\n        ${found.join('\n        ')}`);
});

test('the tray menu speaks every language the web view offers', () => {
  const tray = readFileSync(join(desktop, 'src-tauri/src/tray.rs'), 'utf8');
  const arms = [...tray.matchAll(/^\s+"([a-z]{2})" => \(/gm)].map((m) => m[1]);
  for (const code of languageCodes) {
    if (code === 'en') continue;
    assert.ok(arms.includes(code), `tray.rs has labels for ${code}`);
  }
  assert.match(tray, /tray::relabel|pub fn relabel/, 'the menu can be relabelled');
  const commands = readFileSync(join(desktop, 'src-tauri/src/commands.rs'), 'utf8');
  assert.match(commands, /tray::relabel\(app, &settings\.language\)/, 'save_settings relabels the tray');
});

if (failures > 0) {
  console.error(`\n${failures} i18n check(s) failed`);
  process.exit(1);
}
console.log('\ni18n: every word follows the language');
