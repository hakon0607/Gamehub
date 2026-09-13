/**
 * Every theme's text must be readable against the surfaces it sits on.
 *
 * Measured with the WCAG contrast formula rather than judged by eye: on a dark
 * theme a grey that looks fine on one monitor disappears on another, and there
 * are ten themes to keep straight. 4.5:1 is the threshold for normal-size text.
 *
 * This caught the real thing — every single theme's --ink-faint, the colour of
 * the line under each game title, was below it, one as low as 2.9:1.
 */
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const css = readFileSync(join(here, '..', 'apps/desktop/src/styles.css'), 'utf8');

const themes = [];
for (const m of css.matchAll(/(:root|\[data-theme=['"]([\w-]+)['"]\])\s*\{([^}]*)\}/g)) {
  const name = m[2] ?? 'root';
  const vars = {};
  for (const v of m[3].matchAll(/--([\w-]+):\s*([^;]+);/g)) vars[v[1]] = v[2].trim();
  if (vars.bg || vars.ink) themes.push({ name, vars });
}

const hex = (h) => {
  const s = h.replace('#', '');
  const full = s.length === 3 ? s.split('').map((c) => c + c).join('') : s;
  return [0, 2, 4].map((i) => parseInt(full.slice(i, i + 2), 16));
};
const lum = (rgb) => {
  const [r, g, b] = rgb.map((v) => {
    const c = v / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
};
const ratio = (a, b) => {
  const [x, y] = [lum(hex(a)), lum(hex(b))].sort((p, q) => q - p);
  return (x + 0.05) / (y + 0.05);
};

const PAIRS = [
  ['ink', 'bg'], ['ink', 'bg-card'], ['ink', 'bg-raised'],
  ['ink-dim', 'bg'], ['ink-dim', 'bg-card'], ['ink-dim', 'bg-raised'],
  ['ink-faint', 'bg'], ['ink-faint', 'bg-card'], ['ink-faint', 'bg-raised'],
  ['accent', 'bg'], ['accent', 'bg-card'],
];

const problems = [];
for (const { name, vars } of themes) {
  for (const [fg, bg] of PAIRS) {
    if (!vars[fg] || !vars[bg] || !vars[fg].startsWith('#') || !vars[bg].startsWith('#')) continue;
    const r = ratio(vars[fg], vars[bg]);
    if (r < 4.5) problems.push({ theme: name, fg, bg, ratio: r.toFixed(2), fgHex: vars[fg], bgHex: vars[bg] });
  }
}

problems.sort((a, b) => a.ratio - b.ratio);

if (themes.length < 5) {
  console.error(`  FAIL  only ${themes.length} themes found — the parser is not seeing the stylesheet`);
  process.exit(1);
}

console.log(`  ${themes.length} themes checked against ${PAIRS.length} colour pairs each`);

for (const p of problems) {
  console.error(
    `  FAIL  ${p.theme}: --${p.fg} (${p.fgHex}) on --${p.bg} (${p.bgHex}) is ${p.ratio}:1, needs 4.5:1`,
  );
}

console.log(problems.length === 0 ? '\nall text is readable\n' : `\n${problems.length} unreadable pairs\n`);
process.exit(problems.length === 0 ? 0 : 1);
