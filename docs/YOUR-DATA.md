# Your data, and why an update cannot take it

Everything you build up in GameHub — the games you added by hand, the ones you
hid, the covers you uploaded, your screenshots, your playtime and streaks, your
shortcuts — lives in small files on your own PC. Nothing is uploaded.

You can see exactly where, and take a backup on the spot, under
**Settings → Dine data**.

## Two folders, on purpose

| Folder | What is in it |
|---|---|
| `%APPDATA%\com.gamehub.desktop` | The live data GameHub reads and writes |
| `%APPDATA%\com.gamehub.desktop Backups` | Snapshots |

The backups sit *beside* the data folder rather than inside it, and that is the
whole trick. A Tauri update installs the new version by uninstalling the old one
first, and a Windows uninstaller may delete the application's data folder. If
the backups lived inside it, they would be destroyed by exactly the event they
exist to protect you from. Outside, they survive even a full uninstall — and the
next install finds them again.

## What happens when you update

The first time a new version starts, before it reads or writes anything:

1. It compares the version that last used the folder with its own.
2. If they differ, it copies every state file into a new snapshot named for the
   version it came from — `2026-08-31T09-14-05-update-from-0.2.0`.
3. Only then does it load anything.

So if a new version ever gets something wrong, the previous version's data is
sitting untouched next door, with the date on it. **Settings → Dine data → Hent
tilbake** puts it back, and takes a snapshot of the current state first, so
choosing the wrong one is not a one-way door either.

Update snapshots are never deleted automatically. Manual ones are kept ten deep.

## What happens when a file will not open

This used to be the real danger, and it was silent. If a data file could not be
read, GameHub fell back to an empty one — and then saved that empty version over
the top. A single damaged byte turned into a wiped library, with no message.

Now:

1. The unreadable file is **renamed, never overwritten** — it becomes
   `library.json.unreadable-2026-08-31T09-14-05`, and it stays there.
2. The newest snapshot that *does* open is put back in its place.
3. You are told, in Settings, which file it was and where the original went.

Nothing is deleted at any point in that sequence.

## Keeping your own copy

**Ta sikkerhetskopi nå** in Settings makes a snapshot whenever you want one. To
keep a copy somewhere else entirely — another drive, a USB stick, OneDrive —
open the backups folder from the same screen and copy it wherever you like. It
is ordinary files and folders; there is nothing to export.

## Moving to a new PC

Copy both folders to the same place on the new machine, then install GameHub.
It will find your library, your covers and your shortcuts exactly as you left
them. Screenshots and replay clips live wherever you pointed them — by default
inside the data folder, so they come along too.
