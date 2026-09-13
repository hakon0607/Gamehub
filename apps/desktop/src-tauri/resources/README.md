# resources

`ffmpeg.exe` and `ffprobe.exe` belong here. They are what Replay records and
cuts with, and they are not committed because they are around 80 MB together.

```bash
pnpm fetch-ffmpeg
```

Run that once before building the installer. Without it the build still
succeeds and everything except Replay works; the Replay page explains what is
missing instead of failing silently.

This folder is never empty in the repository: `tauri.conf.json` bundles
`resources/*`, and a glob that matches nothing stops the build. That is why
this file is here.

ffmpeg is licensed under the GPL. What that means for GameHub, and where to
get the source for the exact build that ships, is in
`legal/third-party-notices.md`.
