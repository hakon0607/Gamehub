# GameHub 1.5 — riktig spilletid, bakgrunnsbilder, popup og språk

## Hva som er nytt i 1.5

**Spilletid teller bare når du faktisk spiller.** Før talte klokka så lenge
spillets prosess kjørte — også når du var alt-tabbet til Discord, spillet lå
i bakgrunnen, PC-en sov, eller spillet var frosset. Det er derfor Rainbow Six
sto med nesten seks timer på én dag. Nå spør GameHub Windows hvert tiende
sekund om to ting: hvilket program eier vinduet foran, og når tastatur/mus
sist ble rørt. Tiden teller bare når spillet er vinduet foran *og* du har rørt
noe de siste 10 minuttene (kan endres, eller slås av, i Innstillinger →
Personvern). Frosne spill teller aldri. PC som sover teller aldri (et hull på
over ett minutt mellom to målinger regnes som søvn).

Økten husker fortsatt når spillet ble åpnet og lukket (`wallSeconds`), så
kalenderen kan vise hele kvelden, mens spilletid, streaks og quests bruker
den aktive tiden. Gamle økter beholdes som de er. På Hjem-siden står det
«pause — i bakgrunnen» når spillet er åpent men ikke i front.

## Hva som var nytt i 1.3

**Bakgrunnsbilder** (Innstillinger → Utseende → Bakgrunnsbilder). To store
kort: 🔒 Låseskjerm og 🏠 Skrivebord. Trykk «Endre», velg et bilde fra PC-en
i Windows sin vanlige filvelger (JPG, PNG, WebP, BMP), og GameHub setter det
med en gang. Kortet viser forhåndsvisning, filnavn, oppløsning og størrelse,
og «Aktiv» når Windows faktisk viser bildet ditt. Knapper: Endre, Fjern
(GameHub glemmer valget, skjermen blir som den er), Windows-standard (setter
Windows sitt eget bilde tilbake), og «Bruk skrivebordsbildet på låseskjermen
også» / omvendt. Har du flere skjermer, listes de under «Per skjerm» og kan
få hvert sitt bilde. Bildet kopieres til GameHubs egen mappe, så det
overlever at originalen flyttes, og er valgt fortsatt etter omstart. Ingenting
lastes opp noe sted.

Slik er det gjort, og hva Windows faktisk tillater:

- *Skrivebordet* settes gjennom `IDesktopWallpaper`, det samme grensesnittet
  Innstillinger-appen i Windows bruker. Det virker per skjerm, og GameHub
  kan lese tilbake hva hver skjerm viser — derfor kan kortet si «Aktiv».
- *Låseskjermen* kan et vanlig program bare endre for brukeren som er logget
  inn, gjennom Windows sitt `LockScreen.SetImageFileAsync`. Det bytter
  låseskjermen fra Windows Spotlight til «Bilde». Windows gir ingen måte å
  lese tilbake hvilket bilde som vises, så kortet sier «Sendt til Windows»
  og viser det GameHub sist sendte. Påloggingsskjermen *før* noen er logget
  inn er en maskininnstilling som krever administrator — den rører ikke
  GameHub. En jobb-/skolepolicy kan låse låseskjermen; da nekter Windows,
  og GameHub viser feilen i stedet for å late som.

## Hva som var nytt i 1.2

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

Nå bygger GitHub (Actions-fanen, 10–15 min). Versjonen er 1.5.0, så alle
som har GameHub får popupen «GameHub 1.5.0 er klar».
