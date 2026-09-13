# GameHub — implementation plan for the all-in-one spec

The existing app (v0.1) is the base: two Tauri-free Rust crates
(`game-detection`, `launcher-adapters`), the Tauri app, the React UI, the
Next.js sync/assistant routes. Nothing there is replaced. Everything below is
added to it.

## The architectural keystone

The spec is right that this is the critical decision, so it is the first thing
built and everything else hangs off it:

```
process poller (already exists, one per app)
        │
        ▼
  SessionTracker  ← the only thing that knows what is running
        │
        ├── activity log (sessions, per-day totals)
        │       ├── Calendar
        │       ├── Streaks
        │       └── Recently played
        ├── Quest engine → XP → Level
        ├── Performance page ("current game")
        ├── Replay ("which game is this clip")
        └── Screenshots ("which game is this shot")
```

There is exactly one detector. Everything else *subscribes*. The tracker is a
pure state machine — it takes (running game ids, timestamp) and returns events —
so the whole activity layer is unit-testable without a Windows machine.

## Honest constraints, decided up front

These are the four places where the spec asks for something that cannot be done
the obvious way. Each has a real implementation and a stated limit.

### FPS

Reading a game's frame rate means one of two things:

1. **Injecting a DLL** into the game to hook its present chain. This is what
   RTSS and the Steam overlay do. It is also what anti-cheat systems
   (Vanguard, EasyAntiCheat, BattlEye) are built to detect, and it gets people
   banned. **GameHub will not inject into games.**
2. **ETW frame tracing** — the mechanism Intel's PresentMon uses. The graphics
   driver already emits an event per presented frame; a listener with the right
   privilege can count them. No injection, no game process touched, works with
   anti-cheat.

GameHub uses (2). The cost is honest: it needs administrator rights to start
the trace session, and it is per-process rather than per-swapchain, so a game
with two windows reports the sum. When the trace cannot start, the UI says
"FPS unavailable — GameHub needs to run as administrator" rather than showing a
number.

### The FPS overlay

An overlay *inside* an exclusive-fullscreen game also requires injection. What
works without it is a transparent, click-through, always-on-top window
(`WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE`), which sits over
borderless-windowed and windowed games — the mode most games now default to.

So: the overlay is a real always-on-top window. When the foreground game is in
exclusive fullscreen, GameHub detects that and says
"This game is running in exclusive fullscreen, where Windows will not draw the
overlay. Switch it to borderless windowed." That is section 58's rule applied
literally.

### GPU temperature

- **NVIDIA**: NVML ships with the driver. Real numbers, no admin needed.
- **AMD / Intel**: the equivalents need either a vendor SDK that is not
  redistributable or a kernel driver (what LibreHardwareMonitor installs).
  GameHub ships neither. Those cards show "—" with a tooltip explaining why.

CPU temperature has the same problem on most desktops and gets the same
treatment. CPU/GPU/RAM/disk *usage* are all available without any of this.

### Spotify

OAuth with PKCE, no client secret in the app, no password ever asked for. But
the Web API's playback endpoints (play, pause, next, volume) are
**Premium-only** — a free account can read what is playing and nothing else.
GameHub reads the account tier at connect time and disables the transport
controls with a one-line explanation instead of showing buttons that return 403.

### Replay buffer

Building a ring-buffer recorder from scratch is a project in itself. GameHub
uses Windows Graphics Capture (the API Windows' own Game Bar uses — no
injection) into a hardware encoder, keeping N seconds of encoded segments and
discarding older ones. Hardware encoding is required for this to be free:
NVENC, AMF or Quick Sync. On a machine with none of them, replay is offered at
reduced settings with a clear warning rather than silently eating the CPU.

### XP integrity

On a local-first app, anyone can edit the file. "Prevent users from awarding
themselves XP" therefore means: XP is only ever written by the quest engine
from measured playtime, never by a command the UI can call. The store is
checksummed so tampering is *detectable*, and that is stated plainly rather
than described as tamper-proof.

## Order of work

Matches the spec's own grouping.

| Group | Contents | State |
|---|---|---|
| 1 | Session tracker, activity log, storage | **built** |
| 2 | Recently played, calendar, streaks, XP, quests | **built** |
| 3 | Screenshots, replay buffer, clip library, player | planned |
| 4 | Performance centre, FPS via ETW, overlay window | planned |
| 5 | Quick Tools launcher, clipboard manager | planned |
| 6 | Spotify OAuth and playback | planned |
| 7 | Themes, personalization, dashboard widgets | planned |
| 8 | Shortcut centre, global shortcuts, conflicts | planned |
| 9 | Polish, onboarding, error handling | planned |

Groups 1 and 2 are the ones everything else depends on, which is why they come
first and why they are finished rather than sketched.
