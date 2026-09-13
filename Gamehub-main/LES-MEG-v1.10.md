# GameHub 1.10 — juridisk gjennomgang, vilkår i appen og samtykke

## Hva som er nytt i 1.10

**Innstillinger → Brukervilkår** — en ny, siste seksjon nederst i
innstillingene. Der ligger alt det juridiske samlet:

- **Dokumentene**, lesbare i appen: brukervilkår, personvernerklæring,
  tredjepartslisenser og MIT-lisensen. På norsk og engelsk (de åtte andre
  språkene viser engelsk med vilje — en umerket maskinoversettelse av en
  juridisk forpliktelse er verre enn et fremmedspråk).
- **Hva du har godtatt, og når.** Versjonsnummer og dato, vist tilbake til
  brukeren.
- **Bryteren for bruksstatistikk**, med ID-en din hvis den er på.
- **Eksporter dataene mine** — én JSON-fil med alt appen har om deg på PC-en.
- **Slett statistikken min** — ber serveren slette alt under ID-en din, og
  glemmer så ID-en.
- **Slett mine lokale data** — tømmer bibliotek, spilletid, streaks, oppdrag,
  lister og utklippstavle. Video- og bildefilene blir liggende.

**Statistikken er nå AV som standard.** Dette er den viktigste endringen.
Etter språkvalget kommer en ny skjerm som forklarer kort hva appen gjør, med
lenker til hele teksten, og én avkrysningsboks for statistikken — tom.
«Fortsett» betyr at du godtar vilkårene; statistikken er et eget valg. Grunnen
er ekomloven § 3-15 (ny fra 1. januar 2025): å lagre og lese en ID på
brukerens utstyr krever et samtykke som oppfyller GDPR-standarden. «Alltid på»
gjorde ikke det.

Alle som allerede har appen blir spurt én gang etter oppdateringen. Sier de
nei, sendes ingenting mer — og det opprettes ingen ID i det hele tatt.

**Appen har fått en lisens.** MIT. Den lå aldri der før, samtidig som
nettsiden og videoene sa «open source». Nå stemmer det, og filen følger med
appen. Samtidig er ffmpeg-lisensen dokumentert slik GPL krever, med tilbud om
kildekoden.

**Advarsel om frys.** At «Frys spillet» kan bli oppfattet som juks av
anti-juks-systemer står nå på førstegangsskjermen og i vilkårene. Det er den
funksjonen som kan koste en bruker mest, og det skal ikke være en overraskelse.

## Mappen /legal i koden

Sju arbeidsdokumenter som ikke vises i appen, men som du bør ha:
risikoregister, behandlingsprotokoll (GDPR art. 30), leverandørliste,
lanseringssjekkliste, samtykke og versjonering, notat om
databehandleravtaler, og informasjonskapsler. `legal/README.md` forklarer
hvert enkelt.

Selve vilkårene ligger i `apps/desktop/src/legal/` — altså inne i appen — så
teksten brukeren godtok alltid er nøyaktig den som fulgte med hans versjon.
Nettsiden leser de samme filene fra GitHub.

## Nettsiden

`/terms` og `/privacy` er nye sider som viser de samme dokumentene.
`/api/forget` tar imot sletteanmodningen fra appen. `/api/cleanup` sletter
statistikk eldre enn 24 måneder og kjøres automatisk hver natt — sett
`CRON_SECRET` i Vercel så ingen andre kan trigge den.

## Hva som var nytt i 1.8.1

**Åpningsanimasjon.** Når GameHub åpner på skjermen kommer logoen først:
mørk bakgrunn, et mykt lys som vokser, logomerket som tegnes inn, «GameHub»
som stiger opp, en tynn lyslinje og «Your games. One place.» (på ditt
språk). Så løftes teppet og appen ligger ferdig lastet under. Hele greia
tar rundt to sekunder. Sammen med animasjonen spilles en kort, myk klang
(tre stigende toner) — lyden ligger i `GameHub-startup-sound.mp3` så du kan
høre den før du bygger.

**Når den kommer — og når den ikke kommer.** Åpningen spilles når
GameHub starter, og når vinduet hentes fram igjen etter at du har lukket
det med X (da ligger det i systemkurven, og for deg er det «lukket»). Den
kommer *ikke* når du bare minimerer til oppgavelinjen og henter det opp
igjen — det er ikke å åpne noe. Og aldri når GameHub starter skjult sammen
med Windows, for da er det ingen på skjermen å hilse på.

**Slå av.** Innstillinger → Generelt → «Når GameHub åpner» har tre brytere:
*Åpningsanimasjon*, *Åpningslyd* og *Også når den hentes fram fra
systemkurven*. Slår du av den siste, kommer åpningen bare når selve
programmet starter. Ser du ingen animasjon i det hele tatt: sjekk at den
første bryteren står på.

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

## Slik gir du ut versjonen — tre runder

Samme fremgangsmåte som sist. Den ENESTE filen som starter byggingen er
`apps/desktop/src-tauri/tauri.conf.json`, og den ligger i runde 2.

1. Pakk ut `gamehubv28.zip`. Gjør skjulte filer synlige (Del 2 i
   `OPPSKRIFT.md`). Åpne `gamehub`-mappen på PC-en.
2. **Runde 1:** merk alt i `gamehub`-mappen **unntatt** `apps`-mappen — husk
   `packages` og den nye `legal`-mappen — og dra det inn på forsiden av
   `github.com/hakon0607/Gamehub` → **Add file → Upload files** →
   **Commit changes**.
3. **Runde 2:** klikk `apps`, så `desktop`. Sjekk at adressen slutter på
   `/apps/desktop`. **Add file → Upload files** der, og dra inn mappen
   `src-tauri` pluss de fem løse filene. **Commit changes**.
4. **Runde 3:** stå i `apps/desktop`, **Add file → Upload files**, dra inn
   mappen `src`. **Commit changes**.
5. **Actions** → **Release** → **Run workflow** → huk av **force** → **Run**,
   så bygges 1.10.0 med alt.

Så nettsiden: last opp innholdet i `gamehub-nettside.zip` til
`gamehub-nettside`-repoet. Legg inn `CRON_SECRET` (et langt tilfeldig ord) i
Vercel → Settings → Environment Variables, og Redeploy.

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

