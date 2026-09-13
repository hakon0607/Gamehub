# GameHub 1.2 — popup over spillet, og språk

## Hva som er nytt

**Popup over spillet.** Når GameHub er skjult (i tray) og du trykker en
hurtigtast midt i Fortnite eller et annet spill, dukker et lite kort opp nede
til høyre på skjermen med et pling: «Screenshot lagret», «Klipp lagret ·
Fortnite», «Spillet er frosset», «Replay er på» — og feilen hvis noe gikk
galt. Kortet forsvinner av seg selv etter tre sekunder. Det tar aldri fokus
fra spillet (tastaturet blir i spillet) og museklikk går rett gjennom det.
Popupen og lyden kan slås av hver for seg i Innstillinger → Generelt →
«Popup over spillet».

Ærlig om grensen: spill i **eksklusiv fullskjerm** tegner rett på skjermkortet
og dekker alle vinduer, også denne popupen (samme grense som Discord og Steam
har). Sett spillet til **kantløst vindu** (borderless windowed — standard i
Fortnite og de fleste nye spill), så vises den.


**Språkvalg første gang.** Første gang GameHub starter (og én gang etter
denne oppdateringen, for de som allerede har appen) kommer en skjerm der man
velger språk. Engelsk er forhåndsvalgt. Ti språk: English, Norsk, Svenska,
Dansk, Suomi, Deutsch, Français, Español, Polski, Nederlands.

**Bytt språk når som helst.** Innstillinger → Generelt → Språk. Alt bytter
med en gang: menyer, sider, knapper, innstillinger, feilmeldinger fra
Rust-siden, varsler fra hurtigtastene, oppdrag (quests), datoer og tall,
Hurtigverktøy-vinduet og menyen på ikonet ved klokka.

**Ingenting annet er endret.** Replay, frys, snarveier, oppdatering — alt
virker som i 1.0.2.

## Slik er det bygget (for den som lurer)

- `apps/desktop/src/i18n/en.json` er fasiten: alle ord appen kan si, med
  en nøkkel hver. De ni andre språkfilene har nøyaktig de samme nøklene.
- Rust-siden sier aldri et ord selv. Den sender koder (`@no_game_running`)
  som appen slår opp i språkfilen. Derfor kan ikke en engelsk eller norsk
  feilmelding henge igjen.
- `node scripts/i18n.test.mjs` sjekker alt dette automatisk: at hvert
  språk har alle nøkler, at ingen .tsx-fil har løs tekst, at hver kode
  fra Rust finnes i språkfilen, og at ikonmenyen kan alle språkene. Den
  kjører som en del av `pnpm test:release`, så en glemt oversettelse
  stopper bygget.

## Slik gir du ut versjonen — i tre pakker

GitHub tar imot maks 100 filer per opplasting, derfor er det tre zip-filer.
Last dem opp i rekkefølge, én om gangen. Den siste pakken er den som får
GitHub til å bygge `.exe`-filen, så ingenting starter før alt er på plass.

1. Pakk ut `gamehub-pakke-1.zip`. Gjør skjulte filer synlige (Del 2 i
   `OPPSKRIFT.md`). Åpne `gamehub`-mappen, trykk **Ctrl+A**, og dra alt
   inn på `github.com/hakon0607/gamehub` → **Add file → Upload files**.
   Klikk **Commit changes** nederst.
2. Pakk ut `gamehub-pakke-2.zip`. Åpne `gamehub`-mappen — der ligger bare
   mappen `apps`. Dra `apps`-mappen inn på samme måte. **Commit changes**.
3. Pakk ut `gamehub-pakke-3.zip`. Samme: dra `apps`-mappen inn.
   **Commit changes**.

Nå bygger GitHub (Actions-fanen, 10–15 min). Versjonen er 1.2.0, så alle
som har GameHub får popupen «GameHub 1.2.0 er klar».
