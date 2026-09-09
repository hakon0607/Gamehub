# GameHub

Every PC game you own, in one place. Install it once and Steam, Epic, Xbox, EA,
Ubisoft Connect, Battle.net, GOG and Riot show up in a single
library — including games you install *after* GameHub, without adding them by
hand.

The desktop app is the product. It works with no account and no internet
connection. The website exists only for optional sync and the assistant.

---

## What actually works today

This section is honest about the state of each piece, because a launcher
integration that pretends to work is worse than one that says what it cannot do.

| Launcher | How games are found | Playtime | Notes |
|---|---|---|---|
| **Steam** | `libraryfolders.vdf` + one `appmanifest_*.acf` per game | Last played | Complete. Every library folder on every drive, and the Steamworks redistributable is filtered out. |
| **Epic** | The launcher's own JSON manifests in `%ProgramData%` | — | Complete. DLC, Unreal Engine and plugins are excluded. |
| **GOG** | The `GOG.com\Games` registry keys its installers write | — | Covers Galaxy and offline installers. A folder copied from another PC without installing is not found. |
| **Ubisoft** | `Ubisoft\Launcher\Installs` registry keys | — | Titles come from the install folder — Ubisoft does not store them anywhere readable. Rename once and it sticks. |
| **EA** | Uninstall registry + each game's `installerdata.xml` | — | Games without a content id still appear; Play opens the EA library rather than the game. The app says which ones. |
| **Battle.net** | Uninstall registry entries, `--uid=` from the uninstall command | — | Blizzard's own game list is an undocumented protobuf database that the client holds open; GameHub does not parse it. |
| **Riot** | `RiotClientInstalls.json` + the per-product metadata files | — | Launches via `RiotClientServices.exe` with Riot's documented arguments. |
| **Xbox / Game Pass** | The Gaming Services package registry | — | Windows blocks reading `WindowsApps` itself. Titles are derived from package names and are often abbreviated. |
| **Local folders** | Folders you add yourself | — | One game per subfolder; the executable is picked by name, then by size, ignoring uninstallers and redistributables. |

**Playtime** is only shown where a launcher exposes it without logging into an
account. Steam gives "last played"; GameHub records its own last-played time for
everything it starts.

### Not built yet

Named here rather than stubbed out somewhere in the code:

- **Artwork for a game that is not on Steam at all.** Steam games use their own
  app id; everything else is looked up by title in Steam's public app list, which
  covers most PC games whatever store they came from. A game that was never on
  Steam keeps its initials until you set a cover yourself.
- **The assistant UI.** The API route works (`apps/web/app/api/assistant`); the
  panel inside the desktop app is not built.
- **Cloud sync in the desktop app.** The schema, the conflict-safe push and the
  API route are done; the client that calls them is not.
- **Frame rate, GPU load and temperatures.** Real FPS means hooking the game's
  Direct3D or Vulkan present calls; GPU load and temperatures need NVML, ADL or
  WMI. GameHub shows neither rather than inventing them, and the Performance
  page says so and points at the tools that do it properly.
- **Spotify.** Set aside deliberately.
- **AI-written quest text.** Quests themselves work — they are generated from
  your real library and play history and tracked automatically — but the titles
  and descriptions are written by a local generator, not by a model. Wiring the
  assistant in to reword them is a small change; nothing depends on it.

---

## Security

The app reads your filesystem and starts programs, so the boundary is drawn
tightly:

- **The web view cannot touch the filesystem.** `capabilities/default.json`
  grants no filesystem, shell or HTTP permission at all. There is no `shell`
  plugin in the dependency list.
- **The UI never sends a path or a command line.** It sends a game id. The Rust
  side looks up the launch method it recorded during the scan and re-validates
  it immediately before starting anything.
- **Executables are validated four ways** — allowed extension (`.exe` only,
  never `.bat`, `.cmd` or `.ps1`), inside a folder GameHub already trusts for
  that game, exists, and is a regular file.
- **Only known launcher protocols are opened**: `steam:`,
  `com.epicgames.launcher:`, `uplay:`, `origin2:`, `battlenet:`, `goggalaxy:`
  and `riot:`. Anything else, including `file:`, is refused.
- **Path traversal is refused everywhere** a name comes from outside — artwork
  cache entries included, even though their names are built from digits.
- **Updates are signature-checked** against a public key compiled into the app
  before a downloaded installer is executed.
- **No launcher passwords are stored or asked for**, ever. GameHub reads files
  the launchers already wrote; it does not log into anything.

Each of these has a test that tries to break it.

---

## Activity

Sessions, playtime, streaks and the calendar all come from one place: the
process watcher that already tells the Play button whether a game is running.
When a game's executable appears among the running processes a session opens;
when it disappears the session closes. Nothing else in GameHub tries to work out
whether a game is running.

That single source feeds:

- **Home** — what is playing now, today's total, the current streak, recently
  played, and Game Roulette.
- **Streaks** — current and longest run, gaming days, total playtime, the games
  in the current streak, and the best month. A day counts once you have played
  past a threshold you set; nothing is started or stopped by hand.
- **Calendar** — a month at a glance, shaded by how much was played, with a
  breakdown per day.

Sessions under a minute are discarded, so clicking through a launcher does not
litter the calendar. A session that crosses midnight belongs to the day it
started. All of it lives in `%APPDATA%\GameHub\activity.json` and is never
uploaded; **Settings → Privacy** switches recording off and can clear the
history, and launching games keeps working either way.

## Quests and XP

One daily quest worth 200 XP, two fortnightly worth 500, five monthly worth
1000 — the counts and rewards the spec asks for. They are built from the games
you own and how you have been playing: a favourite you keep returning to, a game
untouched for a month, one you have never started.

Progress is **derived, never stored**. A quest's state is recomputed from the
session log every time the page is read, which means there is nothing to claim,
nothing to forget, and no way to award yourself XP. A quest finished while
GameHub was closed is already finished when it opens. Levels are 10,000 XP
apart.

Generation is deterministic per period, so restarting the app never rerolls the
board, and no two quests on one board are about the same game.

## Replay

GameHub keeps the last 30 seconds to 10 minutes (your choice; 2 minutes by
default) in a rolling buffer and saves it when you press the key — the thing that is hard about instant replay is that you want the
moment *after* it has already happened.

It works by having ffmpeg write short numbered segments into a scratch folder
and letting old ones be overwritten. Pressing save picks the segments covering
the window you asked for and stitches them into one clip, filed under whatever
you are playing. Nothing is hooked into the game and nothing large is held in
memory.

Segments are chosen **by modification time, not by file name**, because the ring
wraps — `seg_0003.mp4` is often newer than `seg_0011.mp4` — and the segment
ffmpeg currently has open is skipped, since it has no moov atom yet and would
break the join. Both have tests.

Three honest limits, all shown in the app rather than discovered later:

- **Exclusive fullscreen cannot be captured.** Desktop capture reads the window
  composition, and a game that owns the display outright is not part of it.
  Borderless windowed works, and every modern game offers it.
- **It costs CPU** while armed. That is what the quality presets are for.
- **Sound is recorded from the default output device** through WASAPI loopback
  (`audio.rs`) and piped into ffmpeg as raw PCM — no "Stereo Mix" or virtual
  cable. A microphone can be mixed in on top. The pump is clock-driven, padding
  silence while the game is quiet, so the audio track never stalls the muxer.

ffmpeg is ~80 MB and not committed. Run `pnpm fetch-ffmpeg` once before building
the installer. Without it the build still succeeds and everything else works —
the Replay page says what is missing instead of failing silently.

## Freeze points

`F7` suspends every process of the running game through `NtSuspendProcess` —
the same mechanism Process Explorer uses — so a cutscene or a dialogue stops on
the frame it is on and resumes from there. At the same moment GameHub grabs the
screen and, when a save folder is known for the game, copies it. Each freeze
point lives beside the replay clips under `Clips\<Game>\Frys_<time>\`.

Limits, stated in the app: a suspended process lives only until the PC restarts
or the game is closed; the save copy is what survives, and can be put back from
the Freeze page. Games protected by anti-cheat refuse the suspend and GameHub
says so. Save folders are found by name in Saved Games, Documents, AppData and
the install folder (`saves.rs`), and confirmed by the user.

## Screenshots

A global hotkey grabs the screen and files the picture under whatever is
running — from the same watcher that drives playtime, never a second guess.
Nothing running files it under "Desktop" rather than crediting the last game
played. Shots are grouped by game, can be favourited, opened full size or
revealed in Explorer, and a file deleted outside GameHub is pruned from the
library rather than left as a broken thumbnail.

A game in exclusive fullscreen hands back an empty frame; GameHub detects that
and says to switch to borderless, instead of saving a black PNG.

## Clipboard history

The last 100 things copied, on this PC and nowhere else — there is no network
code in that module at all. Searchable, pinnable (pinned items are never pushed
out by the cap), and switchable off with one click that also stops the poller.

Text that looks like a credential — `sk-`, `ghp_`, `xoxb-`, a PEM header,
"password:", or a long unbroken random-looking string — is skipped rather than
stored. That is a courtesy, not a guarantee, and the app says so.

## Performance

Real CPU, memory, disk and network from the OS, plus the processes belonging to
the game you are playing. What it will not show is a frame rate, GPU load or a
temperature: see *Not built yet* for why, and the page itself names the tools
that do that job properly.

## Quick Tools

`Ctrl+Space` opens a small always-on-top window over whatever is in front —
including a full-screen game, since it is a separate window rather than the main
one. A search box, arrow keys, Enter. It takes a screenshot, rescans, or copies
one of your recent clipboard items back. Everything it offers is something
GameHub already does; it is a faster way in, not a second implementation.

## Shortcuts

**Settings → Hurtigtaster** lists every shortcut that exists, because the list
is generated from the same registry that registers them — it cannot claim one
that does not exist or miss one that does. Global ones (open GameHub, Quick
Tools, screenshot, overlay) are registered with Windows and work while a game
has focus; the rest only apply in the window, and each row says which it is.
Click a key, press a new combination. Two actions cannot share one: GameHub
names the action already holding it rather than silently overwriting.

## Covers

Covers arrive on their own. Steam games use their app id; games from Epic, Xbox,
GOG, EA, Ubisoft and Battle.net are matched by title against Steam's public app
list, since most PC games are on Steam whether or not you bought them there.
Matching normalises punctuation and edition suffixes, so "Marvel's Spider-Man:
Remastered" finds "Marvel s Spider Man Remastered".

A match by name can be wrong, so those covers are marked **gjettet** on the card
and the game's page says where the picture came from. **Change cover…** opens a
file picker and then a crop tool locked to the 2:3 library shape. The crop happens in the app and the
finished 600×900 PNG is stored with the game, marked as yours so no future scan
replaces it. **Remove my cover** puts it back to whatever can be fetched.

## Updates

Tauri's official updater plugin, called from the front end
(`apps/desktop/src/updater.ts`) — `check()` for the manifest,
`downloadAndInstall()` for the download, `relaunch()` afterwards. There is no
home-made update mechanism anywhere in the app.

GameHub checks eight seconds after launch and shows one dialog if there is
something new: version, release notes, **Oppdater** and **Senere**. "Senere"
silences that version until a newer one appears. Settings has a manual check
that answers "Du har nyeste versjon." when there is nothing new.

Downloads are verified against the public key compiled into the app before
anything is executed, and an update never touches the library, settings or
activity history. `docs/UPDATES.md` is the publishing guide — including the
`pnpm latest-json` helper, since a hand-written manifest is the usual reason
updates silently do nothing.

## Structure

```
gamehub/
├── apps/
│   ├── desktop/            Tauri + React + TypeScript — the product
│   │   ├── src/            The interface
│   │   └── src-tauri/      Commands, launching, tray, watchers
│   └── web/                Next.js — optional sync and assistant APIs
├── packages/
│   ├── shared/             TypeScript types shared by both apps
│   ├── game-detection/     Rust: environment abstraction, VDF parser, path rules, library merge
│   └── launcher-adapters/  Rust: one module per launcher
├── database/schema.sql     Postgres/Supabase schema with row-level security
└── .github/workflows/      CI, and the Windows release build
```

The two Rust crates have **no Tauri dependency**, which is what makes the
launcher detection testable: every adapter runs against a fixture directory tree
and a fake registry, so the whole detection layer is covered by tests that pass
on Linux CI as well as on Windows.

---

## Building it

```bash
pnpm install
pnpm test        # Rust tests + typecheck
pnpm dev         # runs the desktop app
```

### Your data

Everything you build up — manually added games, hidden ones, uploaded covers,
screenshots, playtime, shortcuts — is kept on your own PC and backed up
automatically before a new version runs for the first time. The backups live
*beside* the data folder, not inside it, so an uninstall during an update cannot
take them with it. **Settings → Dine data** shows both folders, lists every
snapshot with its date, and restores one in a click. `docs/YOUR-DATA.md` has the
detail.

### Building the app

`BUILD.bat` in the project root, double-clicked or run from Command Prompt,
produces `GameHub-Setup.exe` and a portable `GameHub.exe` in a `GameHub-ferdig`
folder. It checks Node, pnpm, Rust and the MSVC linker first and stops with a
plain explanation if one is missing. `HOW-TO-BUILD.md` has the detail, in
Norwegian.

A local build has no signing key and does not update itself; releases from
GitHub do.

### Releasing

Publishing is one edit on GitHub: change `version` in
`apps/desktop/src-tauri/tauri.conf.json` and commit. GitHub Actions checks the
version is new, builds the front end, runs every test, builds and **signs** the
installer with a key held as a repository secret, writes `latest.json` and
publishes the release — which every installed GameHub then offers as an update.
No terminal at any point. `docs/RELEASING.md` has the one-time setup.

Commit without changing the version and the run stops with an explanation
rather than releasing the same version twice.

### Building the installer yourself

On a Windows machine, one command:

```bash
pnpm installer
```

It builds the front end, compiles the app in release mode, produces the NSIS
installer and leaves it as:

```
apps/desktop/src-tauri/target/release/bundle/nsis/GameHub-Setup.exe
```

Installing it gives a Start Menu entry under **GameHub**, an entry in
Apps & Features with a publisher and version, and a **"Create Desktop
Shortcut"** checkbox on the installer's final page. The desktop shortcut is
named GameHub and carries the app icon.

The installer asks whether to install for this user or for everyone
(`installMode: "both"`); per-user needs no administrator prompt.

WebView2 is downloaded during installation if the PC does not already have it —
Windows 11 and current Windows 10 do. For an installer that works with no
internet at all, set `bundle.windows.webviewInstallMode.type` to
`offlineInstaller`; it adds about 127 MB.

**No Windows machine?** You do not need one. Push to GitHub, open the Actions
tab, run **Build the Windows installer**, and download `GameHub-Setup.exe` from
the finished run. `docs/DISTRIBUTION.md` covers all three routes, plus what to
expect from SmartScreen when handing the installer to other people.

### Before the first release

1. Replace `OWNER` in `tauri.conf.json` and `apps/web/app/page.tsx` with your
   GitHub username.
2. Optional, for auto-update: follow `docs/UPDATES.md`. It is wired but off by
   default — an updater without a signing key is worse than none.
3. Optional, for sync: create a Supabase project, run `database/schema.sql`,
   deploy `apps/web` to Vercel with `NEXT_PUBLIC_SUPABASE_URL` and
   `NEXT_PUBLIC_SUPABASE_ANON_KEY`.
4. Optional, for the assistant: set `AI_PROVIDER` and `AI_API_KEY` on the Vercel
   project. Gemini and Groq both have free tiers.

---

## How new games are noticed

No drive scanning, ever. GameHub watches the handful of folders launchers write
to when they install something — Steam's `steamapps` on each library drive,
Epic's manifest directory, Riot's metadata folder, and
any folders you added — with a debounced filesystem watcher, plus a slow timer
as a backstop. Installing a game writes hundreds of files; the debounce turns
that into one scan. Idle cost is effectively zero.

Closing the window hides it to the tray, because that is what keeps the
watchers running. Quit properly from the tray menu.
