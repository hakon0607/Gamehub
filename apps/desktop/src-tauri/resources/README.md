# resources

`ffmpeg.exe` belongs here. It is what Replay records with, and it is not
committed because it is around 80 MB.

```bash
pnpm fetch-ffmpeg
```

Run that once before building the installer. Without it the build still
succeeds and everything except Replay works; the Replay page explains what is
missing instead of failing silently.
