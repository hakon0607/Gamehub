# Third-party services

Every service involved in GameHub, what it receives, and the paperwork that
should exist for it.

| Service | Role | What it receives | Processor agreement | Outside EEA? | What to keep on file |
|---|---|---|---|---|---|
| **Vercel Inc.** | Hosts the website, the statistics endpoint and the dashboard | The statistics message, the requesting IP (used for country, not stored by us) | Yes — Vercel's DPA, accepted with the terms of service | US company, EU regions available; SCCs in their DPA | A copy of the accepted DPA and which region the project runs in |
| **Neon Inc.** | Postgres database for the statistics | Everything stored: installation rows, activity, plays, feature counts | Yes — Neon's DPA | Database in EU (Frankfurt); support access from the US under SCCs | A copy of the DPA and a screenshot of the region setting |
| **GitHub, Inc.** (Microsoft) | Source code, releases, installer downloads, update check | The user's IP and version when checking for updates or downloading | Not needed — GitHub is an independent controller for visitors, not a processor for us | US | Nothing beyond the public repository |
| **Microsoft (Outlook/Hotmail)** | The contact mailbox | Whatever users write, including rights requests | Not needed — mail provider acting as controller of the mailbox service | US | Nothing |
| **Gyan Doshi / ffmpeg** | The ffmpeg build shipped in the installer | Nothing — it runs locally | Not applicable | — | The exact build URL and the GPL notice, both in `third-party-notices.md` |
| **Valve (Steam public CDN)** | Cover art for games the user owns | The user's IP and the app ID being fetched | Not applicable — it is the user's own request for a picture | US/global CDN | Nothing |

## Before adding a service

Four questions, in order, and they are quick:

1. **Does the app work without it?** If yes, do not add it. Every provider is
   a new recipient to disclose, a new processor agreement, and a new
   question about transfers.
2. **What personal data does it receive?** If the answer is "none", most of
   the rest disappears.
3. **Is it a processor or an independent controller?** A processor acts on
   your instructions and needs an agreement under Article 28. An independent
   controller (like GitHub for visitors) does not, but must still be named in
   the policy.
4. **Where does the data end up?** Inside the EEA is simple. Outside means
   SCCs, and a line in the privacy policy explaining it.

Anything added has to appear in `DATA-MAP.md`, in the privacy policy, and in
this table — in the same release.

## What is deliberately not used

No analytics on the website, no error tracking service, no advertising or
attribution SDK, no push service, no CDN for user content, no AI provider
unless the user supplies their own key and switches it on. Each was
considered and left out; adding any of them reopens this list.
