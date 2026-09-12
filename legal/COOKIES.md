# Cookies

## In the app

None. A desktop app has no browser storage in the ordinary sense, and
GameHub sets nothing. The one thing it does store on the device — the
installation ID for the statistics — is treated exactly like a cookie under
ekomloven § 3-15 and needs the same consent, which is why it is only created
after the user says yes.

## On the website

One cookie, `gamehub_admin`, and only for the author:

| Property | Value |
|---|---|
| Purpose | Keeps the author logged into the private statistics dashboard |
| Contents | A SHA-256 hash of the admin password, not the password |
| Flags | `httpOnly`, `secure`, `sameSite=lax` |
| Lifetime | 90 days |
| Consent needed? | No — strictly necessary for a login the user explicitly asked for, which is the exception in ekomloven § 3-15 |

Ordinary visitors receive no cookies at all. There is no analytics, no
advertising pixel, no embedded video, no font loaded from a third party and
no social widget — which is why the site has no cookie banner and does not
need one.

## The rule to remember

A cookie banner is not what makes tracking lawful; consent is. The way to
avoid the banner is to avoid the tracking. If anything analytics-like is ever
added to the website, it needs a real consent mechanism first — refusal as
easy as acceptance, nothing loaded before the answer — and this file and the
privacy policy have to be updated in the same change.
