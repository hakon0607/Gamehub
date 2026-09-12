# /legal — the compliance folder

The documents users read **are not here**. They live in
`apps/desktop/src/legal/` and are compiled into the app, so the terms someone
accepted are always exactly the ones that shipped in their version. The
website reads the same files from the repository. One source, no drift —
two versions of a privacy policy that disagree is itself a legal problem.

This folder holds the working papers behind them: the analysis, the record of
processing, the risk register and the launch checklist.

| File | What it is | Read it when |
|---|---|---|
| `RISK-REGISTER.md` | Every identified legal risk, with severity, likelihood and what was done | Before a release that changes what the app collects or does |
| `DATA-MAP.md` | The record of processing activities (GDPR Article 30) | When adding a field, a provider, or a new feature that touches data |
| `THIRD-PARTY-SERVICES.md` | Every service involved, what it receives, and which paperwork is needed | When adding or swapping a provider |
| `LAUNCH-CHECKLIST.md` | Everything to confirm before publishing | Before every release, and in full before a public launch |
| `CONSENT-AND-VERSIONING.md` | How consent and acceptance are recorded and re-asked for | When changing the documents |
| `DPA-NOTE.md` | Why there is no data processing agreement to sign, and when that changes | If a company ever asks for one |
| `COOKIES.md` | The one cookie on the website and why it needs no banner | If anything analytics-like is ever added to the site |

## The user-facing documents

| File in `apps/desktop/src/legal/` | Shown |
|---|---|
| `terms-of-service.en.md` / `.nb.md` | First run, and Settings → Terms of Service |
| `privacy-policy.en.md` / `.nb.md` | Same place, and at webgamehubweb.vercel.app/privacy |
| `third-party-notices.md` | Settings → Terms of Service |
| `license.md` | Copy of the repository LICENSE, shown in the app |

English is authoritative. Norwegian exists because most early users are
Norwegian. The other eight interface languages fall back to English on
purpose: an unchecked machine translation of an obligation is worse than a
document in a foreign language.

## Changing a document

1. Edit the file in `apps/desktop/src/legal/`.
2. Bump `LEGAL_VERSION` and `LEGAL_DATE` in
   `apps/desktop/src-tauri/src/legal.rs`, and the `**Version … — …**` line at
   the top of every English and Norwegian document.
3. Run `pnpm test:release`. `scripts/legal.test.mjs` fails if the versions,
   the contact address, the licence or the list of transmitted fields no
   longer agree.
4. Release. Everyone is asked to accept the new version the next time they
   open the app, and the app records which version and when.

## The honest limit

None of this makes anyone impossible to sue, and none of it is legal advice.
It is a good-faith, documented attempt to follow the rules that apply, to
tell users the truth, and to collect as little as possible. The points marked
**[lawyer]** in `RISK-REGISTER.md` are the ones worth paying a professional
to look at before this gets big.
