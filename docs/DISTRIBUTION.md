# Getting a GameHub installer

The end product is one file, `GameHub-Setup.exe`. Whoever downloads it needs
nothing else — no Node, no pnpm, no Rust, no terminal. It contains the compiled
Rust application and the built front end, and it installs GameHub the way any
Windows program installs.

There are three ways to produce that file. Only the first needs no tools at all.

---

## 1. Let GitHub build it (nothing installed on your machine)

Push this repository to GitHub, then:

1. Open the **Actions** tab.
2. Choose **Build the Windows installer** in the left-hand list.
3. Click **Run workflow** → **Run workflow**.
4. Wait roughly ten minutes. Refresh the run.
5. At the bottom of the finished run there is an **Artifacts** section with
   **GameHub-Setup** — download it and unzip.

Inside is `GameHub-Setup.exe`.

This is the recommended route. It builds on a real Windows machine with the
official Microsoft toolchain, and it runs the full test suite first, so a broken
launcher adapter stops the build instead of shipping.

## 2. Publish a release (the link you send people)

```bash
git tag v0.1.0
git push origin v0.1.0
```

The same build runs and creates a **draft** GitHub Release with
`GameHub-Setup.exe` attached. Open it, check it, press Publish. The public
download link is then:

```
https://github.com/YOUR-USERNAME/gamehub/releases/latest
```

That link is also what a website's Download button should point at — no hosting
of your own required.

## 3. Build it yourself on a Windows PC

Needs Node 22, pnpm, Rust and the Microsoft C++ build tools installed once. Then:

```bash
pnpm install
pnpm installer
```

The result:

```
apps/desktop/src-tauri/target/release/bundle/nsis/GameHub-Setup.exe
```

To rebuild after changing anything, run `pnpm installer` again. It rebuilds the
front end and the Rust app and overwrites that file. Bump `version` in
`apps/desktop/src-tauri/tauri.conf.json` when you release, or Windows will treat
the new build as the same version as the old one.

**This cannot be done on Linux or macOS.** Tauri's own documentation calls
cross-compiling a Windows installer a last resort for when CI is unavailable,
and it needs the Windows Rust standard library, which is not part of a Linux
toolchain. Use route 1.

---

## What the installer does

- Installs into Program Files (or your user folder — it asks which).
- Creates a **GameHub** Start Menu entry. This one is always created.
- Offers a **Create Desktop Shortcut** checkbox on the last page. Ticking it
  puts a GameHub icon on the desktop that launches the app.
- Registers GameHub in **Settings → Apps → Installed apps**, with a publisher,
  a version and a working **Uninstall** button.
- Installs WebView2 if the PC does not have it. Windows 11 and current Windows
  10 already do.

The uninstaller removes the program. It leaves your library, playtime history
and settings in `%APPDATA%\GameHub` alone, so reinstalling picks up where you
left off. Delete that folder by hand for a clean slate.

---

## Before you send it to other people

**Windows will warn them.** An installer that is not code-signed triggers
SmartScreen: *"Windows protected your PC — unknown publisher."* Users have to
click **More info** then **Run anyway**. Nothing is wrong with the file; Windows
simply has no way to tell who built it.

Two options, and both are real:

- **Accept it.** Say so on the download page, with a screenshot of the warning
  and the two clicks. Plenty of small tools ship this way. The warning softens
  over time as more people install without incident.
- **Buy a code-signing certificate.** An OV certificate runs roughly €200–400 a
  year from a certificate authority, and needs identity verification. It removes
  the "unknown publisher" line, though a brand-new certificate still needs
  reputation before SmartScreen goes fully quiet. An EV certificate is more
  expensive and clears SmartScreen immediately. Once you have one, set
  `bundle.windows.certificateThumbprint`, `digestAlgorithm` and `timestampUrl`
  in `tauri.conf.json` and Tauri signs the installer during the build.

There is no free way to remove the warning. Anyone telling you otherwise is
describing a self-signed certificate, which Windows treats as no signature at
all.

Auto-update is a separate thing and is documented in `docs/UPDATES.md`.
