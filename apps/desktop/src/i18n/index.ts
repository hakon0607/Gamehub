/**
 * Every word the interface shows comes through here.
 *
 * `en.json` is the master: its keys are the set of things that can be said.
 * Every other language file carries the same keys (a test enforces it), so a
 * missing translation is a build failure rather than an English word left in
 * a Norwegian screen. `t()` is a plain function on a module-level language so
 * formatting helpers can use it too; the app remounts its tree when the
 * language changes, so nothing has to subscribe.
 *
 * Messages from the Rust side arrive as `@key|param|param` and are turned
 * into text by `tr()`, so the backend never has to know a language.
 */
import en from './en.json';
import nb from './nb.json';
import sv from './sv.json';
import da from './da.json';
import fi from './fi.json';
import de from './de.json';
import fr from './fr.json';
import es from './es.json';
import pl from './pl.json';
import nl from './nl.json';

export type Key = keyof typeof en;
type Dictionary = Record<Key, string>;

export const LANGUAGES: { code: string; name: string; locale: string; dictionary: Dictionary }[] = [
  { code: 'en', name: 'English', locale: 'en-GB', dictionary: en },
  { code: 'nb', name: 'Norsk', locale: 'nb-NO', dictionary: nb as Dictionary },
  { code: 'sv', name: 'Svenska', locale: 'sv-SE', dictionary: sv as Dictionary },
  { code: 'da', name: 'Dansk', locale: 'da-DK', dictionary: da as Dictionary },
  { code: 'fi', name: 'Suomi', locale: 'fi-FI', dictionary: fi as Dictionary },
  { code: 'de', name: 'Deutsch', locale: 'de-DE', dictionary: de as Dictionary },
  { code: 'fr', name: 'Français', locale: 'fr-FR', dictionary: fr as Dictionary },
  { code: 'es', name: 'Español', locale: 'es-ES', dictionary: es as Dictionary },
  { code: 'pl', name: 'Polski', locale: 'pl-PL', dictionary: pl as Dictionary },
  { code: 'nl', name: 'Nederlands', locale: 'nl-NL', dictionary: nl as Dictionary },
];

export const DEFAULT_LANGUAGE = 'en';

let current = LANGUAGES[0]!;

export function setLanguage(code: string): void {
  current = LANGUAGES.find((l) => l.code === code) ?? LANGUAGES[0]!;
  if (typeof document !== 'undefined') document.documentElement.lang = current.code;
  // Anything rendered outside <App /> (the opening splash) re-reads its text on this.
  if (typeof window !== 'undefined') window.dispatchEvent(new Event('language-changed'));
}

export function currentLanguage(): string {
  return current.code;
}

export function locale(): string {
  return current.locale;
}

export type Params = Record<string, string | number>;

function fill(template: string, params?: Params): string {
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (match, name: string) => (name in params ? String(params[name]) : match));
}

/** The text for a key in the current language, with `{name}` filled in. */
export function t(key: Key, params?: Params): string {
  const template = current.dictionary[key] ?? en[key] ?? key;
  return fill(template, params);
}

/** One of two keys by count — `{n}` is filled in either way. */
export function tn(one: Key, other: Key, n: number, params?: Params): string {
  return t(n === 1 ? one : other, { n, ...params });
}

/**
 * A message from the backend: `@key|param0|param1` becomes the translated
 * `be.<key>` with `{0}`, `{1}` filled in. Anything else is shown as it is,
 * which is what an unexpected error from a library looks like.
 */
export function tr(raw: unknown): string {
  const text = raw instanceof Error ? raw.message : String(raw ?? '');
  if (!text.startsWith('@')) return text;
  const [head = '', ...params] = text.slice(1).split('|');
  // `@sc.quick_tools` names a key directly; `@no_game_running` a backend one.
  const key = (head in en ? head : `be.${head}`) as Key;
  if (!(key in en)) return text;
  const filled: Params = {};
  params.forEach((value, index) => {
    // A parameter can itself be a backend message, e.g. a launch error
    // wrapping a reason.
    filled[String(index)] = tr(value);
  });
  return fill(current.dictionary[key] ?? en[key], filled);
}
