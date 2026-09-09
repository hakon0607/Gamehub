# Releasing GameHub

> This is the current setup as of 1.0: a commit that changes the version in
> `tauri.conf.json` publishes a signed release, and installed copies offer it
> as an update. `BUILD.bat` still works for a local, non-updating build.

Once you have done the one-time setup below, publishing a new version is this:

1. Open `apps/desktop/src-tauri/tauri.conf.json` on GitHub.
2. Click the pencil.
3. Change `"version": "0.2.0"` to `"0.2.1"`.
4. Commit.

Ten minutes later there is a signed release, and every installed GameHub offers
the update by itself. No terminal, ever.

---

## One-time setup

Three things, all in a browser. You need your existing private signing key —
the file you created when you first set up updates, at
`C:\Users\<you>\.tauri\gamehub.key`. **Do not create a new one:** every GameHub
already out there only trusts updates signed by that key.

### 1. Copy the private key

Open Notepad. **File → Open**, type `%USERPROFILE%\.tauri\gamehub.key` into the
file-name box and press Enter (set the file-type dropdown to *All Files* if you
do not see it). Select everything with **Ctrl+A** and copy with **Ctrl+C**.

### 2. Add the two secrets

In your repository on GitHub: **Settings → Secrets and variables → Actions →
New repository secret**.

| Name | Value |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | Paste the private key you just copied |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | The password you chose when you created the key |
| `TAURI_UPDATER_PUBKEY` | The contents of `gamehub.key.pub` — see below |

For the third one, open Notepad again: **File → Open**, type
`%USERPROFILE%\.tauri\gamehub.key.pub` and press Enter. That is the *public*
half of the same key pair, written beside the private key when it was created.
Copy all of it.

The public key is kept as a secret not because it is confidential — it is not —
but because it then lives in exactly one place. No file arriving from a zip, a
download or a bad merge can overwrite it, and it does not have to be pasted into
JSON where a stray quote would break the build.

The names have to match exactly. Secrets cannot be read back afterwards, which
is the point — GitHub can use them, nobody can see them, and the key is never in
the repository.

### 3. Let Actions publish releases

**Settings → Actions → General**, scroll to **Workflow permissions**, choose
**Read and write permissions**, and Save. Without this the build succeeds but
cannot create the release.

That is the whole setup.

---

## What happens on every commit

```
You change the version and commit
              ↓
     Check the version          ← stops here if it is not a new one
              ↓
     Build the front end
              ↓
     Run all the tests          ← stops here if anything fails
              ↓
   Build and sign the installer ← uses the secret, never the repository
              ↓
     Write latest.json
              ↓
     Publish the release
```

The version in `tauri.conf.json` drives all of it: the installer's name, the
tag, the entry in Installed apps, and the number the updater compares against.

Four files land on the release:

| File | What it is |
|---|---|
| `GameHub_0.2.1_x64-setup.exe` | The installer |
| `GameHub_0.2.1_x64-setup.exe.sig` | Its signature, which the updater checks |
| `latest.json` | What the updater reads |
| `GameHub-Setup.exe` | The same installer under a name that never changes, so a download link on a website keeps working |

Because the release is marked *latest*, the endpoint the app already points at —
`https://github.com/hakon0607/gamehub/releases/latest/download/latest.json` —
resolves to the new one automatically. That URL never has to change.

---

## When it stops instead of releasing

**"Release v0.2.0 already exists."** You committed without changing the version.
Nothing was published and nothing was broken — bump the version and commit again.
The message tells you the file, the current number and the next one.

**"The version is older than 0.2.0."** A typo, most likely: `0.1.9` instead of
`0.2.1`. GameHub only updates forwards, so an older number would strand
everyone on the version they have.

**"No signature produced."** The installer built but was not signed, which means
`TAURI_SIGNING_PRIVATE_KEY` is missing or the password does not match the key.
Fix the secret and run the workflow again from the **Actions** tab.

**"No updater public key."** The `TAURI_UPDATER_PUBKEY` secret is missing, so the
build stopped before compiling rather than producing an app nobody could update.
The message says where to find the value. Do not create a new key pair — every
GameHub already installed only trusts the key you already have.

**Tests failed.** Nothing is published. The run shows which test, and the
version stays unreleased until it passes — which is the point.

## Rebuilding a version you already released

Rare, but if a release went out broken: **Actions → Release → Run workflow**,
tick **Replace the existing release**, and run it. It deletes the old release
and tag and publishes again from the current code. Anyone who already installed
the broken build gets the replacement as an update.

## Documentation commits

Changes to `.md` files and anything under `docs/` do not trigger a release —
there is no point spending ten minutes building Windows for a typo fix.
