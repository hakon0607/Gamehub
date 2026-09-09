# Get GameHub-Setup.exe without touching a terminal

Five steps, about fifteen minutes, most of it waiting. You need a free GitHub
account and nothing else installed.

---

### 1. Make a repository

Go to **https://github.com/new**

- Repository name: `gamehub`
- Choose **Private** if you like — this works either way
- Leave every checkbox off (no README, no .gitignore, no licence)
- Click **Create repository**

### 2. Upload the project

Unzip `gamehub.zip` somewhere you can find it. You now have a folder called
`gamehub`, with `apps`, `packages`, `docs` and a few files inside it.

On the empty repository page, click **uploading an existing file**.

Open the `gamehub` folder, select **everything inside it** (Ctrl + A), and drag
it onto the upload area. Not the `gamehub` folder itself — the contents.

Wait for the file list to finish appearing, then click **Commit changes**.

> **Check one thing before you commit:** the list must include a `.github`
> entry. Some browsers skip folders starting with a dot. If it is missing, see
> *If .github did not upload* at the bottom — it takes one minute to fix.

### 3. Start the build

Click the **Actions** tab at the top of your repository.

If GitHub asks whether to enable workflows, click the green **I understand my
workflows, go ahead and enable them**.

In the left-hand list, click **Build the Windows installer**.

On the right, click **Run workflow**, then the green **Run workflow** button in
the little panel that appears.

### 4. Wait

A row appears with a yellow dot. It turns into a green tick when the build is
done — usually eight to twelve minutes the first time, faster afterwards.

Click the row to watch it if you like.

### 5. Download the installer

On the finished run's page, scroll to the bottom. Under **Artifacts** there is a
box named **GameHub-Setup**. Click it — a zip downloads.

Open that zip. Inside is:

```
GameHub-Setup.exe
```

Double-click it. Windows will say *"Windows protected your PC"* because the
installer is not code-signed — click **More info**, then **Run anyway**. That
warning is covered in `docs/DISTRIBUTION.md`, including how to remove it.

The installer asks where to install, and on its **last page** there is a
checkbox:

```
☑ Create Desktop Shortcut
```

Leave it ticked. Click **Finish**.

You now have **GameHub** on your desktop with the GameHub icon. Double-click it
and the application opens. No terminal, ever again.

---

## Building a new installer after you change something

Repeat steps 3 to 5. Upload the changed files first if you edited anything
locally.

Before releasing a new version to other people, open
`apps/desktop/src-tauri/tauri.conf.json` on GitHub, click the pencil icon and
change `"version": "0.1.0"` to the new number. Windows uses that to tell builds
apart.

## Giving it to other people

Instead of step 3, create a tag — but that needs git. The simpler way: after
downloading the installer, go to your repository's **Releases** page, click
**Create a new release**, invent a tag like `v0.1.0`, drag `GameHub-Setup.exe`
into the attachments box and publish. The download link is then
`https://github.com/YOUR-NAME/gamehub/releases/latest` and you can send it to
anyone.

## If .github did not upload

On your repository, click **Add file** → **Create new file**. In the filename
box type exactly:

```
.github/workflows/release.yml
```

(GitHub turns the slashes into folders as you type.)

Open `.github/workflows/release.yml` from the unzipped folder in Notepad, copy
everything, paste it into the big box on GitHub, and click **Commit changes**.

Then go back to step 3.
