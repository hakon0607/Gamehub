# Adding a launcher

Every adapter is one file in `packages/launcher-adapters/src/`, implementing
`LauncherAdapter`. It gets an `Env` — known folders plus a registry reader — and
returns an `AdapterScan`.

Two rules, and both are the reason the existing adapters can be trusted:

**Take the path from the machine, not from a constant.** Registry value first,
manifest second, well-known default last and only if it exists on disk. A
hard-coded `C:\Program Files (x86)\...` is wrong for anyone with a second drive.

**Never invent a capability.** If a launcher does not expose playtime, leave the
field `None`. If it can only be read partially, set `limitation` — the Settings
page shows that string to the user verbatim.

Then write the test *first*, as a fixture: build the directory tree and the
`FakeRegistry` the way a real machine looks, and assert on the games that come
out. `steam.rs` and `epic.rs` are the two worth copying from. Because the crate
has no Tauri dependency, these tests run on Linux, which means CI catches a
broken parser before anyone installs the build.

# One build-order rule

`tauri::generate_context!()` checks that `frontendDist` exists **while the macro
expands**, not at runtime. So anything that compiles `apps/desktop/src-tauri` —
`cargo check`, `cargo test`, `cargo build` — fails with

> The `frontendDist` configuration is set to `../dist` but this path doesn't exist

unless the front end has been built first. `tauri build` and `tauri dev` handle
this themselves through `beforeBuildCommand`; a bare cargo command does not.

`build.rs` now writes a clearly-labelled placeholder `index.html` when that
folder is missing, so a bare cargo command works on a fresh clone and CI cannot
trip over the ordering again. Vite empties and replaces the directory on a real
build, so the placeholder never reaches an installer.

You still want the real front end when you are actually running the app:

```bash
pnpm --filter @gamehub/desktop build
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml
```

Both GitHub workflows build the front end first as well.
