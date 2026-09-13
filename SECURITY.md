# Security policy

## Reporting a vulnerability

Please report security problems privately to **hakon.solvik@hotmail.com**
rather than opening a public issue. Include what you found, how to reproduce
it, and how you would like to be credited.

You will get an acknowledgement within **7 days** and an assessment within
**30 days**. Confirmed problems are fixed in the next release, and the release
notes say that a security issue was fixed without describing how to exploit
it until most users have updated.

Please do not run tests that degrade the service for others, access data that
is not yours, or keep any personal data you come across.

## What is in scope

- The GameHub desktop app (this repository)
- The statistics endpoint and dashboard at webgamehubweb.vercel.app
- The release and update mechanism

Out of scope: the hosting providers themselves (report those to Vercel, Neon
or GitHub), and social engineering.

## Supported versions

Only the newest released version is supported. The app updates itself, so
fixes reach users by installing the update it offers.

## If personal data is involved

A breach involving personal data is assessed against GDPR Article 33 and, where
it is likely to result in a risk to people, reported to the Norwegian Data
Protection Authority within 72 hours of discovery. See the privacy policy.
