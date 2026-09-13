/**
 * The legal documents that ship with the app.
 *
 * The files live in /legal at the top of the repository, so there is one
 * copy of every document: the app compiles it in, the website reads the same
 * file from GitHub, and the two can never drift apart.
 *
 * They are bundled as text, not fetched: the terms a user accepted must be
 * readable with no internet, and must be exactly the ones that shipped in
 * the version they are running. English is authoritative; Norwegian is
 * offered because most early users are Norwegian. Other languages fall back
 * to English rather than to a machine translation nobody has checked —
 * a mistranslated obligation is worse than a foreign-language one.
 */
import termsEn from '../../../legal/terms-of-service.en.md?raw';
import termsNb from '../../../legal/terms-of-service.nb.md?raw';
import privacyEn from '../../../legal/privacy-policy.en.md?raw';
import privacyNb from '../../../legal/privacy-policy.nb.md?raw';
import thirdParty from '../../../legal/third-party-notices.md?raw';
import licence from '../../../legal/license.md?raw';

export type LegalDoc = 'terms' | 'privacy' | 'thirdParty' | 'licence';

/** Which documents exist in which language. */
const TEXTS: Record<LegalDoc, { en: string; nb?: string }> = {
  terms: { en: termsEn, nb: termsNb },
  privacy: { en: privacyEn, nb: privacyNb },
  thirdParty: { en: thirdParty },
  licence: { en: licence },
};

/** The order they are listed in, and the `legal.*` key for each title. */
export const DOCS: { id: LegalDoc; key: string }[] = [
  { id: 'terms', key: 'legal.doc_terms' },
  { id: 'privacy', key: 'legal.doc_privacy' },
  { id: 'thirdParty', key: 'legal.doc_third_party' },
  { id: 'licence', key: 'legal.doc_licence' },
];

/** The document in the user's language, or English when there is no version. */
export function legalText(doc: LegalDoc, language: string): string {
  const texts = TEXTS[doc];
  return (language === 'nb' && texts.nb) || texts.en;
}

/** True when the user is reading English because their language has no copy. */
export function isFallback(doc: LegalDoc, language: string): boolean {
  return language !== 'en' && !(language === 'nb' && TEXTS[doc].nb);
}
