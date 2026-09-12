# Legal risk register — GameHub

Assessed 12 September 2026 against the code as it stands in version 1.10.0.
Severity is the damage if it goes wrong; likelihood is the chance it does.
Items marked **[lawyer]** depend on facts or on unsettled law and should be
reviewed by a professional before GameHub gets a large user base.

This register reduces risk. It does not remove it, and nothing here is a
promise that no one can bring a claim.

## Resolved in this release

| # | Risk | Why it mattered | Severity | Likelihood | What was done |
|---|---|---|---|---|---|
| 1 | Statistics sent with no consent and no way to opt out | A persistent identifier stored on and read back from the user's device is personal data; ekomloven § 3-15 requires GDPR-standard consent and analytics is not "strictly necessary". No valid basis under Article 6 either | High | High | Consent is now asked once, off by default, refusal as easy as acceptance, withdrawable in Settings. No ID is created and no request made without it |
| 2 | No LICENSE while marketed as "open source" | Inaccurate marketing (markedsføringsloven), nobody could lawfully reuse the code, and the CRA free-software exemption assumes actual free software | Medium | High | MIT licence added, shipped in the app, shown in Settings, checked by a test |
| 3 | Bundled ffmpeg is GPLv3 with no notice and no source offer | Distributing a GPL binary carries obligations even when it stays a separate program | Medium | Medium | `third-party-notices.md` names the exact build, ships the licence, and carries a written offer of source. A test fails if the download URL changes without the notice changing |
| 4 | No way for a user to see or delete their data | Articles 15, 17 and 20 answered only by email, from a mailbox that may not be watched | Medium | Medium | "Export my data", "Delete my statistics data" and "Delete my local data" in Settings → Terms of Service; `/api/forget` deletes server-side rows |
| 5 | No retention limit on the statistics | "As long as we like" is not a retention period | Medium | High | 24 months, enforced daily by a cron job, written in the policy |
| 6 | Freeze and overlay may trigger anti-cheat bans | Users can lose game accounts worth real money; the feature is marketed | High | Medium | Warned on the first-run screen, in the terms (section 6) and in the tutorial. Risk allocated to the user, liability disclaimed as far as Norwegian law allows |
| 7 | Publisher shown as "GameHub", a non-person | A privacy policy must name a real controller | Low | High | Håkon Solvik named as controller and publisher, with a contact address, in every document |
| 8 | Recording captures more than the game | Clips can contain other people's faces and messages | Medium | Medium | Stated in the terms (section 7) and the privacy policy; everything stays local |

## Open — accepted, watched, or needing advice

| # | Risk | Why it matters | Severity | Likelihood | Position |
|---|---|---|---|---|---|
| 9 | The name "GameHub" has not been cleared | Other products use it. A trademark holder can demand a rename after the brand is built | Medium | Medium | **[lawyer]** Search Patentstyret and EUIPO before investing more in the name. Cheap now, expensive later |
| 10 | Marketing videos use real cover art | Using publishers' artwork in your own promotion is a different question from showing a user their own library | Medium | Medium | **[lawyer]** before any paid advertising. Organic posting is lower risk but not zero |
| 11 | Per-install game names | The most intrusive field sent: it builds a play history against a persistent ID | Medium | Low | Kept, with consent and full disclosure. Consider aggregating it away if the user base grows |
| 12 | Digital content act (digitalytelsesloven) | May apply where digital content is supplied against personal data instead of money | Medium | Low | **[lawyer]** Making the statistics genuinely optional is the strongest available answer: the app is supplied without counter-performance |
| 13 | Cyber Resilience Act | Reporting duties began 11 September 2026. Free software outside commercial activity is exempt | Medium | Low | Exempt as long as GameHub stays free and non-commercial with a real open-source licence. **Re-assess the day money is involved** |
| 14 | Clipboard history on by default, stored in clear text | Passwords get copied. The file is readable by anything running as that user | Medium | Medium | Disclosed; can be switched off and wiped. **Consider defaulting it off** in a future release |
| 15 | A minor as controller | If the named controller is under 18, limited legal capacity affects binding terms and liability, and their name and address sit in a public document | Medium | — | **[lawyer]** If Håkon is under 18, name an adult (a parent) as the responsible person, or have one co-sign. Easy to change: the name is in the documents and one test |
| 16 | Game EULAs and launcher terms | Launching and reading other companies' software is normal interoperability, but some EULAs restrict automation | Low | Low | The app only reads public manifest files and starts the launcher's own executable. No circumvention, no modification |
| 17 | Update server and endpoint availability | Users depend on updates for security fixes | Low | Medium | Stated in the terms: the app keeps working, it just stops updating |
| 18 | The dashboard password is the only protection on real data | One shared secret guards the statistics | Medium | Low | Use a long unique password, never reused. Consider a second factor if the data grows |

## Deliberately out of scope

No payments, no accounts, no user-generated content, no marketplace, no app
store. That removes the right of withdrawal, consumer purchase law,
subscription and auto-renewal rules, refund policy, community guidelines,
content moderation, DSA obligations and store compliance. **The day any of
those changes, this register and the documents have to be reopened.**
