# Record of processing activities

Modelled on GDPR Article 30. A single controller who is not a company and
processes no special categories is not strictly required to keep one, but it
is the cheapest way to answer a question from a user or Datatilsynet, and it
makes every later change easy to judge.

**Controller:** Håkon Solvik, private individual, Norway ·
hakon.solvik@hotmail.com
**Last reviewed:** 12 September 2026 (GameHub 1.10.0)

## 1. Data that never leaves the user's PC

| Item | Purpose | Where | Controller? |
|---|---|---|---|
| Game library, install paths, cover art | Show the library, launch games | `%APPDATA%\GameHub\library.json` and the artwork folder | No — the author never receives it |
| Playtime sessions, streaks, quests, XP | Statistics the user asked for | `activity.json` | No |
| Replay clips, screenshots, freeze points, copied save folders | The features themselves | The user's chosen folders | No |
| Clipboard history (optional, on by default) | Recall what was copied | `clipboard.json` | No |
| Settings, including language, theme, chosen wallpaper | Run the app as configured | `settings.json` | No |
| Metadata API keys, AI API key, if the user enters them | Call services on the user's behalf | `settings.json` | No — and they are excluded from the data export |

Nothing in this table is transmitted. The author has no access and no way to
obtain it. It is listed because users ask what the app stores, and because
the export and delete functions must cover all of it.

## 2. Anonymous usage statistics — the only processing the author controls

- **Purpose:** know how many people use GameHub, which versions are live,
  which features matter, and which languages are used.
- **Legal basis:** consent, GDPR Article 6(1)(a). Storing and reading the
  installation ID on the device additionally requires consent under the
  Norwegian Electronic Communications Act § 3-15.
- **Data subjects:** people who install GameHub and switch the statistics on.
- **Categories:** installation ID (random, pseudonymous), app version,
  interface language, window state, name of the game running, number of
  installed games and their launchers, Windows version, per-feature counters.
  Country derived from the request; IP address discarded, never stored.
- **Special categories:** none. No profiling and no automated decisions.
- **Recipients:** Vercel Inc. (hosting, processor), Neon Inc. (database,
  processor, EU/Frankfurt). No one else. Not sold, not used for advertising.
- **Transfers:** database in the EU. Provider support access from the US is
  covered by Standard Contractual Clauses and, where certified, the EU–US
  Data Privacy Framework.
- **Retention:** installation rows, daily activity and play rows deleted 24
  months after the last message. Daily feature totals carry no identifier and
  are kept indefinitely. Enforced by `/api/cleanup`, run daily by a Vercel
  cron job.
- **Security:** HTTPS; the endpoint accepts only the listed fields; the
  dashboard is password-protected with an httpOnly, secure cookie holding a
  hash; database credentials only in the hosting provider's environment.
- **User controls:** on/off in Settings → Terms of Service; "Delete my
  statistics data" deletes every row under the ID and forgets the ID.

## 3. Update check

- **Purpose:** offer the user a newer version.
- **Legal basis:** legitimate interest, Article 6(1)(f) — users benefit from
  security fixes, and the request carries nothing but a version number.
- **Recipient:** GitHub, Inc., which sees the request and the IP address
  under its own privacy notice. The author receives nothing and keeps nothing.
- **Retention:** none by the author.

## 4. Email

- **Purpose:** answering questions, access and deletion requests, security
  reports.
- **Legal basis:** legal obligation (Article 6(1)(c)) for rights requests;
  legitimate interest otherwise.
- **Recipient:** Microsoft (Outlook/Hotmail), as the mailbox provider.
- **Retention:** rights requests kept for 2 years as proof they were handled,
  then deleted.

## 5. What is deliberately not collected

No account, no email address at install time, no name, no device
fingerprint, no advertising ID, no crash dumps containing memory, no file
paths, no contents of clips, screenshots or the clipboard, no game account
identifiers, no IP address in storage. Each of these was considered and left
out because the app works without it.
