# Third-party notices

GameHub is published under the MIT licence (see LICENSE). It ships and uses
components from other people, under their own licences. This file is the
notice those licences require. It is written in English only, as is customary
for licence notices.

## ffmpeg — GPLv3

The Windows installer includes **ffmpeg.exe** and **ffprobe.exe**, which
GameHub calls as separate programs to record and cut replay clips. The build
that ships is:

- **ffmpeg 7.1 "essentials" build for Windows**, published by Gyan Doshi
- Downloaded from
  https://github.com/GyanD/codexffmpeg/releases/download/7.1/ffmpeg-7.1-essentials_build.zip
- That build includes libx264 and is therefore licensed under the
  **GNU General Public License, version 3 or later (GPLv3+)**.

GameHub does not link ffmpeg into its own program. It starts ffmpeg as a
separate process and talks to it over the command line and standard input, so
the two remain separate works and GameHub keeps its own MIT licence. The
ffmpeg binary itself stays under the GPL, and these obligations apply to it:

- The full text of the GPLv3 ships with the installer as `LICENSE-ffmpeg.txt`
  and is published at https://www.gnu.org/licenses/gpl-3.0.html
- **Written offer of source code:** the complete corresponding source for the
  ffmpeg build distributed with GameHub is available from
  https://github.com/GyanD/codexffmpeg and from https://ffmpeg.org/download.html.
  If you want the source for the exact build shipped in a given GameHub
  release and cannot obtain it there, write to **hakon.solvik@hotmail.com**
  and you will be sent it, or a link to it, at no charge beyond the cost of
  distribution.
- ffmpeg is a trademark of Fabrice Bellard, originator of the FFmpeg project.

If you build GameHub yourself without running `pnpm fetch-ffmpeg`, no GPL
component is included and only the MIT licence applies.

## Rust crates and npm packages

GameHub is built on the Tauri framework and a long list of open-source
libraries, nearly all under the MIT or Apache-2.0 licences. The authoritative,
machine-generated list for a given release is produced by:

```
cargo tree                     # Rust dependencies
pnpm licenses list             # JavaScript dependencies
```

Their licence texts are reproduced in the files those commands reference in
the repository. No copyleft library is linked into GameHub itself.

## Game titles, cover art and launcher names

Steam, Epic Games Store, Xbox, EA app, Ubisoft Connect, Battle.net, GOG and
Riot Client are the trademarks of their respective owners. Game titles and
cover art belong to their publishers.

GameHub is not affiliated with, endorsed by or sponsored by any of them. These
names and images are used only to identify software the user already owns and
has installed — the smallest use needed for the app to do its job.

Cover art is fetched from the launchers' own files on the user's PC and, where
that fails, from Valve's public content network. It is cached on the user's PC
and never redistributed by the author.

Rights holders who want material removed should write to
**hakon.solvik@hotmail.com**. See section 8 of the Terms of Service.

## Fonts and icons

The interface uses the fonts that ship with Windows (Segoe UI Variable) with
system fallbacks. No font files are redistributed. The icons in the interface
are Unicode characters, not licensed artwork.

## Sounds

The chime played at start-up and the notification "pling" were generated for
this project with a short Python script and are covered by GameHub's own MIT
licence.
