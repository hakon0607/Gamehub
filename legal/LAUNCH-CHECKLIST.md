# Legal launch checklist

Mark each line: `[ ]` not started · `[~]` in progress · `[x]` done ·
`[L]` needs a lawyer.

Status below is how things stood when this was written, on 12 September 2026.

## A. The basics — done in code

- [x] MIT licence in the repository, shipped in the app, shown in Settings
- [x] Terms of Service, English and Norwegian, in the app and on the website
- [x] Privacy policy, English and Norwegian, in the app and on the website
- [x] Third-party notices, including the ffmpeg GPL notice and source offer
- [x] `SECURITY.md` with a reporting address and response times
- [x] A named, reachable controller in every document
- [x] Terms accepted on first run, recorded with version and timestamp
- [x] Statistics consent asked separately, off by default, refusal as easy
- [x] Consent withdrawable in Settings, effective from the next tick
- [x] Export my data (Article 15/20) without contacting anyone
- [x] Delete my statistics data (Article 17), server-side
- [x] Delete my local data
- [x] Retention limit of 24 months, enforced by a daily cron job
- [x] Anti-cheat warning on the first-run screen and in the terms
- [x] Automated test that the documents, the code and the licence agree

## B. Before you publish this release

- [ ] Read the Terms of Service and the privacy policy from start to finish
      yourself. You are the one who has to stand behind them
- [ ] Confirm the name and the email address are the ones you want in public,
      permanently and findable
- [ ] Set `CRON_SECRET` in Vercel so `/api/cleanup` cannot be triggered by
      anyone else
- [ ] Confirm the Neon database region is inside the EU
- [ ] Set a long, unique `ADMIN_PASSWORD` that is used nowhere else
- [ ] Install the build on a clean machine and check: the terms appear, "no"
      to statistics really sends nothing, export writes a file, delete works
- [ ] Update the website (`/terms` and `/privacy` must show version 1.0)
- [ ] Say in the release notes that the statistics are now opt-in, and that
      anyone who had them on before is asked again

## C. Marketing, before you post anywhere

- [ ] Remove "open source" from anywhere it was said before the licence
      existed — or leave it, now that it is true
- [ ] Mention the SmartScreen warning in the download description so nobody
      thinks it is malware
- [ ] Do not use publisher cover art in paid advertising — [L] if you plan to
- [ ] Do not use launcher logos in thumbnails; names in plain text are safer

## D. Worth doing soon

- [L] Trademark search for "GameHub" in Patentstyret and EUIPO before the
      name is worth anything
- [L] If the named controller is under 18, name an adult as responsible
- [ ] Decide whether clipboard history should default to off
- [ ] Consider dropping per-install game names once there are enough users
      for aggregates to be meaningful

## E. The day anything changes

Reopen the documents and the risk register when any of these becomes true:

- [ ] Money is involved — payment, donations, ads, a paid tier. That brings
      consumer law, the right of withdrawal, subscription rules, and it ends
      the Cyber Resilience Act exemption for free software
- [ ] Accounts or cloud sync arrive — new personal data, new basis, new
      security duties, deletion of an account to build
- [ ] Users can publish anything to each other — moderation, notice and
      takedown, and DSA questions
- [ ] The app starts sending anything not listed in the privacy policy
- [ ] A new provider appears — see `THIRD-PARTY-SERVICES.md`
- [ ] An AI feature is switched on by default rather than by the user

## F. If something goes wrong

- [ ] Personal data exposed: write down what happened and when you found out,
      assess the risk to people, and if there is one, notify Datatilsynet
      within 72 hours (datatilsynet.no has the form)
- [ ] A rights request arrives: answer within 30 days, ask for the
      installation ID, and keep a note that you answered
- [ ] A takedown or trademark complaint arrives: do not argue on social
      media, answer by email, and get advice before promising anything
