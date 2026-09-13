# GameHub 1.0 — hva som er nytt, og hvordan du publiserer

Dette er en ny versjon av GameHub bygget på nytt fra grunnen: samme datastruktur
for spill, quests, aktivitet, klipp og innstillinger som før (alt du har
lagret leses rett inn), men ny kode i appen, nytt grensesnitt og tre store
funksjonsendringer.

## Hva som er nytt

**Replay tar opp lyd.** Spillyden — det du hører i høyttalerne eller
hodetelefonene — tas opp gjennom Windows sin egen loopback (WASAPI) og pipes
inn i ffmpeg sammen med videoen. Ingen «Stereo Mix», ingen virtuell kabel.
Det er på som standard. En mikrofon kan i tillegg velges under Innstillinger →
Replay → Lyd, og blandes da inn i samme lydspor.

**Replay husker opptil 10 minutter.** Bufferlengden velges fritt fra 30
sekunder til 10 minutter med en glidebryter, både på Replay-siden og i
Innstillinger. Standard er 2 minutter. Diskbruken vises ved siden av. Hvor mye
F8 lagrer velges separat, og på Replay-siden kan du lagre 15 sek, 30 sek,
1 min, 2 min, 5 min eller 10 min med ett klikk.

**«Frys spillet nå» (F7).** Midt i en cutscene, en dialog eller et oppdrag
der spillet ikke lar deg lagre: trykk F7. Alle spillets prosesser suspenderes
av Windows, så spillet stopper på nøyaktig det bildet og fortsetter derfra når
du trykker F7 igjen (eller «Fortsett» i appen). Samtidig tas et bilde av
skjermen og — hvis en lagringsmappe er valgt for spillet — en kopi av
lagringsmappen. Frysepunktene ligger i samme mappestruktur som replay-klippene:
`Clips\<Spillnavn>\Frys_<tidspunkt>\` med `bilde.png`, `save\` og `frys.json`.

Ærlig om grensen: en frosset prosess lever bare så lenge PC-en er på. Omstart
eller at spillet lukkes avslutter frysen. Lagringskopien er den delen som
overlever, og den kan legges tilbake fra Frys-siden («Legg tilbake lagringen»)
når spillet er avsluttet. Spill med anti-juks (EasyAntiCheat, BattlEye,
Vanguard) beskytter prosessene sine og kan ikke fryses — GameHub sier fra.

Lagringsmappen velges på hvert spills side: «Finn» leter i Saved Games,
Documents, AppData og installasjonsmappen etter noe som heter det samme som
spillet og lar deg velge; «Velg…» lar deg peke selv.

**Nytt grensesnitt.** Mørkeblått og svart, animert bakgrunn, knapper som
lyser og løfter seg, kort som glir inn. Navigasjonen er delt i Spill, Opptak,
Fremgang og Verktøy. Innstillinger har tolv kategorier med et eget søkefelt,
og **Ctrl+K** åpner et søk som finner spill, sider, innstillinger og handlinger
uansett hvor du er — skriv «lyd» så havner du rett på lydinnstillingen.
Hurtigverktøyet over spillet (Ctrl+Space) har fått Frys, Fortsett og Lagre
klipp.

**Automatiske oppdateringer er på igjen.** Appen sjekker GitHub 8 sekunder
etter oppstart og viser en popup med «Oppdater nå» / «Senere» når det finnes
en nyere versjon. Oppdateringen er signert og sjekkes mot nøkkelen før den
installeres. Se under.

Alt annet er beholdt: alle ni launchere, automatisk oppdaging, covere, quests
og XP, streaks, kalender, screenshots (F9), utklippstavle, ytelse,
hurtigtaster, sikkerhetskopier, «Dine data», tray, Quick Tools.

## Slik publiserer du (alt i nettleseren)

### Én gang: de tre hemmelighetene

Repoet: **Settings → Secrets and variables → Actions → New repository secret**.

| Navn | Verdi |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | Innholdet i `%USERPROFILE%\.tauri\gamehub.key` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Passordet du valgte da nøkkelen ble laget |
| `TAURI_UPDATER_PUBKEY` | Innholdet i `%USERPROFILE%\.tauri\gamehub.key.pub` |

Hvis du la dem inn i en tidligere versjon står de der fortsatt — hemmeligheter
forsvinner ikke selv om filene i repoet byttes ut. Sjekk at alle tre er listet.
Åpne filene med Notisblokk (Fil → Åpne, lim inn stien, velg «Alle filer»).
**Ikke lag en ny nøkkel:** installerte GameHub-er stoler bare på oppdateringer
signert med den gamle.

Sjekk også **Settings → Actions → General → Workflow permissions → Read and
write permissions**.

### Hver gang: last opp, og bump versjonen

1. Pakk ut zip-en. Slå på **Vis → Skjulte elementer** i Utforsker, så
   `.github`, `.gitignore` og `.npmrc` blir med (se `UPLOAD-TO-GITHUB.md`).
2. På GitHub: **Add file → Upload files**, dra inn alt, **Commit changes**.
   Denne opplastingen inneholder `"version": "1.0.0"` i
   `apps/desktop/src-tauri/tauri.conf.json`, så Release-workflowen starter
   av seg selv og lager `v1.0.0`.
3. Neste gang du vil publisere: åpne `apps/desktop/src-tauri/tauri.conf.json`
   på GitHub, klikk blyanten, endre `"version": "1.0.0"` til `"1.0.1"`,
   commit. Ti–femten minutter senere ligger det en signert release med
   `GameHub-Setup.exe`, og alle installerte GameHub-er tilbyr oppdateringen.

En commit som ikke endrer versjonsfilen starter ingen release (bare CI). En
commit som endrer filen uten å øke versjonen stopper med en forklaring.

Lokalt bygg med `BUILD.bat` fungerer fortsatt, men en slik bygg har ingen
nøkkel og oppdaterer seg ikke selv — Innstillinger → Oppdateringer sier fra.

## Filene som er nye eller skrevet om

| Fil | Hva |
|---|---|
| `apps/desktop/src-tauri/src/audio.rs` | Systemlyd via WASAPI-loopback, klokkestyrt pumpe inn i ffmpeg |
| `apps/desktop/src-tauri/src/replay.rs` | Opptaket: lydinnganger, miksing, 30 s–10 min buffer, ekte ffmpeg-tester |
| `apps/desktop/src-tauri/src/freeze.rs` | Suspender/gjenoppta prosesser (NtSuspendProcess) |
| `packages/game-detection/src/freeze.rs` | Frysepunkt-indeksen, testet på Linux |
| `packages/game-detection/src/saves.rs` | Finne lagringsmapper og kopiere dem |
| `apps/desktop/src/**` | Hele grensesnittet, skrevet på nytt |
| `scripts/preview.mjs` | Viser appen i en nettleser med falsk backend og tar skjermbilder |
