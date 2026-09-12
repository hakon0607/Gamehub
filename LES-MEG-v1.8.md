# GameHub 1.8 — åpningsanimasjon med lyd

## Hva som er nytt i 1.8

**Åpningsanimasjon.** Når GameHub åpner på skjermen kommer logoen først:
mørk bakgrunn, et mykt lys som vokser, logomerket som tegnes inn, «GameHub»
som stiger opp, en tynn lyslinje og «Your games. One place.» (på ditt
språk). Så løftes teppet og appen ligger ferdig lastet under. Hele greia
tar rundt to sekunder. Sammen med animasjonen spilles en kort, myk klang
(tre stigende toner) — lyden ligger i `GameHub-startup-sound.mp3` så du kan
høre den før du bygger.

**Hver gang du åpner den.** GameHub ligger som regel og kjører i
systemkurven (ved klokka) hele tiden, så «å åpne GameHub» betyr i praksis
å hente vinduet fram igjen — fra ikonet ved klokka, med hurtigtasten «Åpne
GameHub», eller ved å trykke på GameHub i Start-menyen mens den allerede
kjører. Åpningen spilles i alle disse tilfellene, over den ferdig lastede
appen. Den spilles derimot ikke når GameHub starter skjult sammen med
Windows — da er det ingen på skjermen å hilse på.

**Slå av.** Innstillinger → Utseende → «Når GameHub åpner» har to brytere:
*Åpningsanimasjon* og *Åpningslyd*. De er uavhengige, så du kan ha
animasjonen uten lyd.

## Slik er det bygget

- `apps/desktop/src/Splash.tsx` er teppet. Det ligger over appen fra første
  bilde, spør Rust-siden «skal jeg?» (`startup_greeting`), og løfter seg med
  en gang hvis svaret er nei — ingenting blinker.
- Rust-siden svarer ja én gang per oppstart, aldri ved `--tray`. Hver gang
  vinduet hentes fram fra skjult (`commands::open_main`, brukt av
  systemkurv-menyen, hurtigtasten og «én instans»-sperren) sender den
  hendelsen `opening`, og teppet spilles på nytt. Lyden spilles (`assets/startup.wav`) selv gjennom Windows sin PlaySound —
  samme vei som plinget fra popupen, så det trenger ingen lydtillatelse.
- To nye innstillinger: `startupAnimation` og `startupSound` (begge på).
  Tekster finnes på alle ti språk (`settings.g_opening`,
  `settings.startup_*`, `splash.tagline`).

## Slik gir du ut versjonen — én pakke, tre opplastinger

**Først: rydd opp etter sist.** I repoet ligger det nå en mappe `desktop`
og en mappe `web` rett på forsiden (ved siden av `apps`). De havnet der
fordi de ble dratt inn på forsiden i stedet for inne i `apps`. Byggingen
ser aldri på dem. Slett dem slik: klikk på `desktop` på forsiden → klikk
på en hvilken som helst fil → trykk de tre prikkene `···` oppe til høyre
→ **Delete directory** → **Commit changes**. Gjør det samme med `web`.

Så selve opplastingen. `gamehubv25.zip` inneholder alt. GitHub tar imot
maks 100 filer per opplasting, og `apps`-mappen alene er 105 filer —
derfor tre runder:

1. Pakk ut `gamehubv25.zip`. Gjør skjulte filer synlige (Del 2 i
   `OPPSKRIFT.md`). Åpne `gamehub`-mappen på PC-en.
2. **Runde 1 (75 filer):** merk ALT i `gamehub`-mappen **unntatt**
   `apps`-mappen og dra det inn på forsiden av
   `github.com/hakon0607/gamehub` → **Add file → Upload files** →
   **Commit changes**.
3. **Runde 2 (48 filer):** på GitHub, klikk på `apps`, så på `desktop`.
   Sjekk at adressen øverst slutter på `/apps/desktop` — **dette er
   det viktige steget.** Trykk **Add file → Upload files** der. På PC-en
   åpner du `gamehub/apps/desktop` og drar inn mappen `src-tauri` pluss
   de fem løse filene (`.gitignore`, `index.html`, `package.json`,
   `tsconfig.json`, `vite.config.ts`). **Commit changes**. Byggingen
   starter nå av seg selv.
4. **Runde 3 (47 filer):** stå fortsatt i `apps/desktop` på GitHub, trykk
   **Add file → Upload files**, og dra inn mappen `src` fra
   `gamehub/apps/desktop` på PC-en. **Commit changes**.
5. Gå til **Actions**-fanen. Fordi byggingen startet i runde 2, før
   `src` var på plass, må den kjøres én gang til: klikk **Release** i
   lista til venstre → **Run workflow** → huk av **force** → **Run
   workflow**. Den bygger nå 1.8.0 med alt.

Sjekk at det gikk riktig: `apps/desktop/src/Splash.tsx` skal finnes på
GitHub, og `apps/desktop/src-tauri/tauri.conf.json` skal si `"1.8.0"`.

## Hva som var nytt i 1.6

**Klipp til replay-klipp.** Åpne et klipp på Replay-siden → **Klipp til**. En
linje under videoen viser hvor du er. Spol dit du vil starte og trykk
**Start her**, spol dit du vil slutte og trykk **Slutt her**, og
**Forhåndsvis** spiller akkurat den biten. **Lagre som nytt klipp** lager en
ny fil ved siden av originalen; **Erstatt originalen** sletter originalen.
Kuttet lander på nøyaktig bilde (videoen kodes på nytt, lyden kopieres).

**Fjern hurtigtast helt.** Innstillinger → Hurtigtaster → **Fjern** på en
tast, eller trykk Backspace mens den lytter. Da har handlingen ingen tast i
det hele tatt.

**Spilletid.** Etter ditt ønske: klokka teller hele tiden spillet er åpent
(standard). Men aldri mens PC-en sover, og frosne spill teller aldri. Vil du
heller telle bare når spillet er vinduet foran, slå på «Tell bare aktiv
spilling» under Innstillinger → Personvern. Gamle økter over 12 timer (spill
som sto åpent over natta, logget av eldre versjoner) slettes én gang ved
første oppstart — det er der 23-timersdagen kom fra.

## Hva som var nytt i 1.5

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

