# gamehub-web

The download site for GameHub: what the app is, the current version, every
release with its notes, and an admin page to publish new ones.

Next.js on Vercel, with Vercel Blob for the files and the release list. No
database.

`SETUP.md` has the setup, in Norwegian.

## How it fits together

The whole dataset is one JSON blob, `releases.json`, holding the release list;
the installers are blobs beside it. There is no database because there is
nothing a database would add: a handful of releases, written by one person, read
on every page load.

Uploads go **from the browser straight to Blob**. This is the one piece of the
design that is not negotiable: a Vercel function refuses a request body over
about 4.5 MB, so a 90 MB installer posted to the server fails every time. The
server only signs a short-lived token, after checking the session.

## Auth

A signed, httpOnly cookie. The password is compared on the server; every route
that changes anything verifies the signature. A check in the browser would be
decoration — the upload API is reachable directly whatever the page does.

Credentials come from `ADMIN_USERNAME` and `ADMIN_PASSWORD`, defaulting to
`admin` / `admin123`. The admin page shows a warning while the default is in
use, because `/admin` is a guessable URL and whoever gets in can publish an
executable that other people download and run.

## Development

```bash
npm install
npm test        # version ordering, sessions, formatting
npm run build
npm run dev
```

The tests transpile the TypeScript with the compiler already in the project, so
there is no framework to install.

## Layout

```
src/lib/         version ordering, sessions, the manifest — the tested parts
src/app/api/     login, logout, session, upload token, release list
src/app/         the pages
```
