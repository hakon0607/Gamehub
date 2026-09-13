# Oppdateringer

GameHub bruker Tauri sin offisielle updater. All koden ligger i frontend og
kaller `@tauri-apps/plugin-updater` direkte — det finnes ingen egenlaget
oppdateringsmekanisme noe sted.

| Fil | Rolle |
|---|---|
| `apps/desktop/src/updater.ts` | `check()` og `downloadAndInstall()` fra plugin-en, pluss `relaunch()` |
| `apps/desktop/src/components/UpdateDialog.tsx` | Popup-en med **Oppdater** og **Senere** |
| `apps/desktop/src/App.tsx` | Den stille sjekken 8 sekunder etter oppstart |
| `apps/desktop/src/views/Settings.tsx` | «Se etter oppdateringer» |
| `apps/desktop/src-tauri/src/lib.rs` | Registrerer updater- og process-plugin-en |
| `apps/desktop/src-tauri/capabilities/default.json` | `updater:default` og `process:allow-restart` |

---

## Bygge og publisere en ny versjon

### 1. Sett versjonen

`apps/desktop/src-tauri/tauri.conf.json`:

```json
"version": "0.2.1"
```

Dette er eneste sted versjonen står. Rust-krates, installeren, oppføringen i
Installerte apper og tallet updateren sammenligner mot — alt leser herfra.

### 2. Legg signeringsnøkkelen i miljøet

PowerShell, i vinduet du skal bygge fra:

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content "$env:USERPROFILE\.tauri\gamehub.key" -Raw
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "passordet ditt"
```

Uten disse lager ikke Tauri noen `.sig`, og da kan ingen oppdatere til bygget.

### 3. Bygg

```bash
pnpm installer
```

### 4. Lag latest.json

```bash
pnpm latest-json "Kort tekst om hva som er nytt"
```

Den leser versjon, signatur og filnavn rett ut av bygget og skriver
`latest.json` ved siden av installeren. Teksten du gir den er det brukerne ser i
popup-en.

### 5. Publiser

Lag en release med taggen `v0.2.1` og last opp **alle tre**:

```
GameHub_0.2.1_x64-setup.exe
GameHub_0.2.1_x64-setup.exe.sig
latest.json
```

---

## Viktig om v0.2.0

Releasen din har `.exe` og `.sig`, men **ikke `latest.json`**. Endepunktet
peker på

```
https://github.com/hakon0607/gamehub/releases/latest/download/latest.json
```

så uten den filen får GameHub 404 og finner aldri noen oppdatering — uansett hvor
riktig koden er. Legg den til på v0.2.0-releasen, eller bare ta den med fra og
med neste versjon.

## Slik oppfører den seg

1. Åtte sekunder etter oppstart — etter at vinduet er tegnet og første
   spillskanning er ferdig — henter GameHub `latest.json`.
2. Er det ikke noe nyere, eller er det ikke nett, skjer ingenting i det hele
   tatt. Ingen dialog, ingen feilmelding.
3. Er det noe nyere: én dialog, **«Ny oppdatering tilgjengelig»**, med
   versjonsnummer, utgivelsesnotatene fra `latest.json`, og knappene
   **Oppdater** og **Senere**.
4. **Senere** husker den versjonen, og den blir ikke tilbudt igjen før du
   publiserer noe enda nyere.
5. **Oppdater** laster ned, sjekker signaturen mot den offentlige nøkkelen som
   er kompilert inn i appen, avviser hvis den ikke stemmer, installerer og
   starter GameHub på nytt.

Innstillinger har i tillegg **Se etter oppdateringer**, som svarer «Du har
nyeste versjon.» når det ikke er noe nytt. Oppstartssjekken sier aldri fra om
det — en uoppfordret «ingenting nytt»-boks er nettopp avbrytelsen ingen vil ha.

Ingenting i `%APPDATA%\GameHub` røres av en oppdatering: bibliotek, covers,
spilletid, streaks, quests og innstillinger overlever.

## Når noe går galt

| Situasjon | Hva brukeren ser |
|---|---|
| Ingen nett | «GameHub kunne ikke nå oppdateringsserveren. Sjekk internettforbindelsen og prøv igjen.» |
| Signaturen stemmer ikke | «Oppdateringen bestod ikke signatursjekken og ble ikke installert.» |
| `latest.json` mangler | «Fant ingen latest.json for denne versjonen ennå.» |
| Bygget uten `pubkey` | «Oppdateringer er ikke satt opp i denne bygget av GameHub.» |

En avbrutt eller feilet oppdatering etterlater aldri en halvinstallert GameHub —
den som kjører fortsetter til en verifisert installer er klar til å erstatte den.
