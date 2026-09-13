# GameHub 1.1 — språk

## Hva som er nytt

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

## Slik gir du ut versjonen

Samme som sist: pakk ut zip-en, gjør skjulte filer synlige, og last opp
alt til `github.com/hakon0607/gamehub` med **Add file → Upload files**
(Del 1, 2 og 4 i `OPPSKRIFT.md`). Versjonstallet i zip-en er 1.1.0, så
GitHub bygger `.exe`-filen av seg selv, og alle som har GameHub får
popupen «GameHub 1.1.0 er klar».
