# How consent and acceptance are recorded

Consent that cannot be proved is not much use, and consent that cannot be
withdrawn is not consent. This is the mechanism, in one page.

## What is stored, and where

In `settings.json` on the user's own PC, under `legal`:

| Field | Meaning |
|---|---|
| `termsVersion` | The version of the terms the user accepted, e.g. `1.0` |
| `termsAcceptedAt` | When, ISO 8601 |
| `statsConsent` | `true` only if the user actively said yes |
| `statsConsentVersion` | The document version on screen when they chose |
| `statsConsentAt` | When the choice was last made, either way |

Nothing about consent is stored on a server. That is deliberate: a server-side
consent log would itself be personal data about people who may have said no.
The record lives with the person it concerns, and the app shows it back to
them in Settings → Terms of Service.

## The rules this respects

- **Separate.** Accepting the terms and consenting to statistics are two
  actions. Bundling them would make the consent invalid (GDPR Article 7(2)).
- **Unticked.** The statistics box starts empty. Pre-ticked boxes are not
  consent.
- **Symmetrical.** Continuing without ticking is one click, exactly like
  continuing with it ticked. Nothing nags afterwards.
- **No penalty.** Every feature behaves identically either way.
- **Withdrawable.** One switch in Settings, and sending stops at the next
  five-minute tick. Withdrawal is as easy as giving it.
- **Provable.** Version and timestamp, shown to the user.

## Version bumps

`LEGAL_VERSION` in `apps/desktop/src-tauri/src/legal.rs` is the single source
of truth. When the accepted version is not the current one, the first-run
screen appears again, with wording that says the terms have changed.

Bump it for anything that changes what the app does with data or what the
user is agreeing to. Do not bump it for a typo — re-asking everyone for a
comma teaches people to click through without reading, which defeats the
point.

When the privacy policy changes in a way that affects the statistics, the
consent is asked again rather than carried over.

## What happens on refusal

No installation ID is generated. No file records one. No request is made. The
dashboard never learns that the install exists. This is why the ID is created
on consent rather than at first start: an identifier that exists "just in
case" is still an identifier stored on someone's device.
