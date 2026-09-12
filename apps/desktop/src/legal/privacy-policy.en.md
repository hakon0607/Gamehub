# Privacy Policy

**Version 1.0 — 12 September 2026**

This explains what GameHub does with information about you. The short version:
GameHub runs on your PC, keeps your data there, and sends nothing about you
anywhere unless you switch on usage statistics yourself. It is off until you
say yes.

## 1. Who is responsible

**Håkon Solvik**, a private individual in Norway, decides what GameHub does
with personal data and is therefore the controller under the GDPR.

Email: **hakon.solvik@hotmail.com**

There is no data protection officer. GameHub is a free project and is not
required to have one. Write to the address above for anything on this page.

## 2. What stays on your PC and never leaves it

Almost everything. GameHub stores the following in its own folder under your
Windows user profile, and none of it is uploaded:

- your game library: titles, install folders, which launcher each game belongs
  to, cover art, favourites and hidden games;
- playtime: when a session started and ended, and how long you played;
- streaks, quests and experience points;
- screenshots, replay clips and freeze points, including the copied save
  folders;
- clipboard history, if you leave that feature on;
- your settings, including the language, the theme and the picture you chose
  as a wallpaper.

This data is not sent to the author and not to anyone else. It is not
encrypted on disk beyond the protection Windows gives your user account, so
anyone who can log in as you can read it. Treat it the way you treat the rest
of your documents.

## 3. What is sent, and only if you say yes

GameHub can send **anonymous usage statistics** so the author can see whether
anyone uses the app and which features matter. This is **switched off until
you turn it on**. You are asked once, on first start, with "no" as easy to
choose as "yes", and you can change your mind at any time in
**Settings → Terms of Service**.

If you turn it on, then every five minutes while GameHub runs it sends:

| What | Example | Why |
|---|---|---|
| A random installation ID | `9f2a…` (32 random characters) | So two messages from the same PC are not counted as two people |
| App version | `1.9.3` | To see who has updated |
| Language | `nb` | To know which translations are used |
| Window state | `open` or `tray` | To tell real use from the app sitting idle |
| The game running right now | `Rocket League` | To see which games people play with GameHub |
| Number of installed games and their launchers | `14`, `steam, epic` | To know what a typical library looks like |
| Windows version | `Windows 11 (26100)` | To know what to test on |
| Feature counts since the last message | `screenshot: 2` | To see which features are worth keeping |

**What is never sent:** your name, your email, your Windows user name, file
paths, the contents of clips, screenshots or the clipboard, your game
accounts, or anything you typed.

**Your IP address** reaches the server as part of any internet request — that
is how the internet works. It is used to derive the country your request came
from and is **then discarded**. The IP address itself is never written to the
database.

The installation ID is a random number created on your PC. It is not derived
from your hardware, your Windows installation or anything else about you, and
it is not linked to any other data set. It is still treated as personal data
under the GDPR, because it lets messages from your PC be recognised over time.

## 4. Legal basis

| Processing | Basis |
|---|---|
| Everything stored on your own PC | No basis needed: the author does not receive it and is not the controller for what stays on your machine |
| Usage statistics | **Your consent**, GDPR Article 6(1)(a), and the consent required by the Norwegian Electronic Communications Act § 3-15 for storing and reading the installation ID on your device |
| Update check | **Legitimate interest**, Article 6(1)(f): telling you about a fixed security problem. This request goes to GitHub, not to the author |

Consent is voluntary. Everything in GameHub works exactly the same if you say
no, and nothing is nagged, hidden or degraded because you did.

## 5. Withdrawing consent

Turn the switch off in **Settings → Terms of Service**. Sending stops
immediately.

Turning it off stops future messages but does not by itself delete what was
already sent. Use **Delete my statistics data** in the same place: the app
sends your installation ID once more with a request to delete everything
stored under it, then forgets the ID. After that nothing links any remaining
row to you, and a fresh ID is only created if you later say yes again.

## 6. How long data is kept

| Data | Kept |
|---|---|
| Everything on your PC | Until you delete it. Playtime and clips are kept until you remove them; you can wipe all of it from the app |
| Statistics: installation rows | 24 months after the last message from that installation, then deleted |
| Statistics: daily activity and play rows | 24 months |
| Statistics: feature counts per day | Indefinitely — these are totals with no ID attached and are no longer personal data |
| Server request logs at the hosting provider | Short-term, as set by the provider, for security and troubleshooting |

## 7. Who else is involved

The statistics service uses two providers. Both act on the author's
instructions as data processors, and both offer data processing terms that
cover this use:

- **Vercel Inc.** — hosts the website and the endpoint that receives the
  messages. Vercel is American with EU regions and infrastructure.
- **Neon Inc.** — hosts the database where the statistics are stored. The
  database is located in the **EU (Frankfurt)**.

In addition, **GitHub, Inc.** hosts the source code and the installer, and
serves the update check. When your app checks for updates, GitHub sees the
request, including your IP address, under its own privacy notice.

Nobody buys this data, it is not used for advertising, and it is not shared
with anyone else. The author is the only person who reads the dashboard, which
is behind a password.

## 8. Transfers outside the EEA

The database is in the EU. Vercel and GitHub are American companies, so
support staff may in principle access systems from outside the EEA. These
transfers rest on the European Commission's **Standard Contractual Clauses**
and, where the provider is certified, the **EU–US Data Privacy Framework**.

This is an area where the law keeps moving. If you would rather not be part of
it at all, simply leave the statistics off — then nothing about you is sent
anywhere.

## 9. Your rights

Under the GDPR you can ask to:

- **see** what is stored about you (access);
- **correct** something that is wrong;
- **delete** it (erasure);
- **limit** what is done with it;
- **object** to it;
- **get a copy** in a machine-readable format (portability);
- **withdraw consent** at any time.

Two of these you can exercise yourself, immediately, without asking anyone:
**Settings → Terms of Service** has **Export my data** (a JSON file with
everything GameHub holds on your PC) and **Delete my statistics data**.

For anything else, email **hakon.solvik@hotmail.com**. To answer at all, the
request needs your installation ID — it is shown in
**Settings → Terms of Service**. Without it there is no way to find your rows,
and no way to know they are yours. Answers come within 30 days.

You can complain to the Norwegian Data Protection Authority
(**Datatilsynet**, datatilsynet.no) or to the authority in the EEA country
where you live.

## 10. Children

GameHub is for users aged 13 and over. Users under 16 need a parent's or
guardian's permission before turning on usage statistics. No age is stored,
and no attempt is made to identify children — asking for a date of birth would
mean collecting more data than the app otherwise needs.

If you are a parent and believe your child has switched the statistics on,
write to the address above and it will be deleted.

## 11. Security

- The statistics endpoint accepts only the fields listed above and rejects
  anything else; the messages travel over HTTPS.
- The dashboard is behind a password; the session cookie stores a hash, not
  the password, and is httpOnly and marked secure.
- The database is reached with credentials that live only in the hosting
  provider's environment variables, never in the source code.
- The app asks Windows for no permission it does not use, and has no remote
  control channel: nobody can make your copy of GameHub do anything from
  outside.

If you find a security problem, please report it to the address above rather
than publishing it. See SECURITY.md in the repository.

## 12. If something goes wrong

If personal data is exposed by accident, the incident is assessed and, where
it is likely to put people at risk, reported to Datatilsynet within 72 hours
of it being discovered. If the risk to you is high, you are told directly in
the app and on the website.

## 13. Cookies

The app uses no cookies and no advertising or tracking technology. The
website, **webgamehubweb.vercel.app**, sets one cookie, and only for the
author: a login cookie for the private dashboard. It is strictly necessary for
that login and requires no consent. Ordinary visitors get no cookies at all,
and the site carries no analytics.

## 14. Changes

This policy carries a version number. If it changes in a way that matters, the
app tells you the next time you open it, and asks again before anything new is
sent. Older versions are kept in the repository so you can see what changed.

## 15. Contact

**Håkon Solvik**
Email: hakon.solvik@hotmail.com
Privacy questions, access requests and deletion requests: same address.
