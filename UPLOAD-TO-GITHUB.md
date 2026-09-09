# Uploading this project to GitHub

Read this first. One Windows setting causes almost every "Actions did not run"
problem, and it is not obvious.

## The problem: Windows hides files that start with a dot

This project contains files and folders whose names begin with a `.`:

```
.github/workflows/release.yml     the release automation
.github/workflows/ci.yml          the tests
.gitignore                        keeps build junk out of the repository
.npmrc                            pnpm settings
.env.example                      template for optional cloud settings
```

File Explorer does not show these by default. If you unzip this project and drag
the files into GitHub, everything in that list is silently left behind — and
because `.github/workflows/` is what GitHub Actions reads, **nothing happens
when you commit.** That is the whole failure, and it looks like a broken zip.

## The fix: two clicks, no terminal

In File Explorer, open the **View** menu, go to **Show**, and tick
**Hidden items**.

- Windows 11: **View → Show → Hidden items**
- Windows 10: the **View** tab on the ribbon → tick **Hidden items**

The `.github` folder and the rest appear immediately. Now select everything and
upload as usual.

## Uploading

1. Unzip this file somewhere, e.g. your Desktop.
2. Turn on **Hidden items** as above.
3. On your repository page, click **Add file → Upload files**.
4. Open the unzipped folder, press **Ctrl+A** to select everything, and drag it
   into the browser. Confirm you can see `.github` in the list before you drop.
5. Scroll down and click **Commit changes**.

## If you would rather not fight Explorer

Any file can be created directly in the browser, and GitHub makes the folders
for you:

1. **Add file → Create new file**.
2. Type the path as the filename, slashes included:
   `.github/workflows/release.yml`. Each `/` you type turns the part before it
   into a folder.
3. Paste the file contents, then **Commit changes**.

This works for `.gitignore` and `.npmrc` too.

## How to tell it worked

Your repository's file list should show a `.github` folder, and the **Actions**
tab should list two workflows: **CI** and **Release**. If the Actions tab is
empty, `.github/workflows/` did not make it up — go back to the hidden-items
setting.

## Before the first release

Three repository secrets, at **Settings → Secrets and variables → Actions**.
`docs/RELEASING.md` explains each one and where to find its value:

| Secret | What it is |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | Your existing private signing key |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | The password for that key |
| `TAURI_UPDATER_PUBKEY` | The public half, from `gamehub.key.pub` |

Then **Settings → Actions → General → Workflow permissions → Read and write
permissions → Save**, so Actions is allowed to create releases.

After that, publishing a version is one edit: change `version` in
`apps/desktop/src-tauri/tauri.conf.json` and commit.
