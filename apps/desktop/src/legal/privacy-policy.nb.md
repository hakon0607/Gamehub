# Personvernerklæring

**Versjon 1.0 — 12. september 2026**

Her står det hva GameHub gjør med opplysninger om deg. Kortversjonen: GameHub
kjører på PC-en din, beholder dataene der, og sender ingenting om deg noe sted
med mindre du selv slår på bruksstatistikk. Den er av til du sier ja.

## 1. Hvem som er ansvarlig

**Håkon Solvik**, privatperson i Norge, bestemmer hva GameHub gjør med
personopplysninger og er dermed behandlingsansvarlig etter GDPR.

E-post: **hakon.solvik@hotmail.com**

Det finnes ikke personvernombud. GameHub er et gratisprosjekt og er ikke
pålagt å ha det. Skriv til adressen over om alt på denne siden.

## 2. Det som blir på PC-en din og aldri forlater den

Nesten alt. GameHub lagrer følgende i sin egen mappe under Windows-brukeren
din, og ingenting av det lastes opp:

- spillbiblioteket: titler, installasjonsmapper, hvilken launcher hvert spill
  hører til, coverbilder, favoritter og skjulte spill;
- spilletid: når en økt startet og sluttet, og hvor lenge du spilte;
- streaks, oppdrag og erfaringspoeng;
- skjermbilder, replay-klipp og frysepunkter, inkludert kopierte save-mapper;
- utklippstavlehistorikk, hvis du lar den funksjonen stå på;
- innstillingene dine, inkludert språk, tema og bildet du valgte som
  bakgrunnsbilde.

Disse dataene sendes ikke til opphavspersonen og ikke til noen andre. De er
ikke kryptert på disken utover den beskyttelsen Windows gir brukerkontoen din,
så alle som kan logge inn som deg kan lese dem. Behandle dem som du behandler
resten av dokumentene dine.

## 3. Det som sendes, og bare hvis du sier ja

GameHub kan sende **anonym bruksstatistikk** slik at opphavspersonen kan se om
noen bruker appen og hvilke funksjoner som betyr noe. Dette er **avslått til
du slår det på**. Du blir spurt én gang, ved første oppstart, der «nei» er
like lett å velge som «ja», og du kan ombestemme deg når som helst under
**Innstillinger → Brukervilkår**.

Slår du det på, sendes dette hvert femte minutt mens GameHub kjører:

| Hva | Eksempel | Hvorfor |
|---|---|---|
| En tilfeldig installasjons-ID | `9f2a…` (32 tilfeldige tegn) | Så to meldinger fra samme PC ikke telles som to personer |
| Appversjon | `1.9.3` | For å se hvem som har oppdatert |
| Språk | `nb` | For å vite hvilke oversettelser som brukes |
| Vindustilstand | `open` eller `tray` | For å skille reell bruk fra at appen står i ro |
| Spillet som kjører akkurat nå | `Rocket League` | For å se hvilke spill folk bruker GameHub sammen med |
| Antall installerte spill og hvilke launchere | `14`, `steam, epic` | For å vite hvordan et typisk bibliotek ser ut |
| Windows-versjon | `Windows 11 (26100)` | For å vite hva som må testes |
| Funksjonstellere siden forrige melding | `screenshot: 2` | For å se hvilke funksjoner som er verdt å beholde |

**Det som aldri sendes:** navnet ditt, e-posten din, Windows-brukernavnet
ditt, filstier, innholdet i klipp, skjermbilder eller utklippstavle,
spillkontoene dine, eller noe du har skrevet.

**IP-adressen din** når serveren som en del av enhver internettforespørsel —
slik fungerer internett. Den brukes til å utlede hvilket land forespørselen
kom fra, og **forkastes deretter**. Selve IP-adressen skrives aldri til
databasen.

Installasjons-ID-en er et tilfeldig tall laget på PC-en din. Den er ikke
utledet fra maskinvaren din, Windows-installasjonen din eller noe annet om
deg, og den kobles ikke til noe annet datasett. Den regnes likevel som en
personopplysning etter GDPR, fordi den gjør at meldinger fra din PC kan
gjenkjennes over tid.

## 4. Rettslig grunnlag

| Behandling | Grunnlag |
|---|---|
| Alt som lagres på din egen PC | Trenger ikke grunnlag: opphavspersonen mottar det ikke og er ikke behandlingsansvarlig for det som blir på maskinen din |
| Bruksstatistikk | **Ditt samtykke**, GDPR artikkel 6 nr. 1 bokstav a, og samtykket ekomloven § 3-15 krever for å lagre og lese installasjons-ID-en på utstyret ditt |
| Oppdateringssjekk | **Berettiget interesse**, artikkel 6 nr. 1 bokstav f: å fortelle deg om et rettet sikkerhetsproblem. Forespørselen går til GitHub, ikke til opphavspersonen |

Samtykket er frivillig. Alt i GameHub fungerer nøyaktig likt om du sier nei,
og ingenting maser, skjules eller forringes fordi du gjorde det.

## 5. Å trekke tilbake samtykket

Slå av bryteren under **Innstillinger → Brukervilkår**. Sendingen stopper
umiddelbart.

Å slå den av stopper fremtidige meldinger, men sletter ikke i seg selv det som
allerede er sendt. Bruk **Slett statistikken min** samme sted: appen sender
installasjons-ID-en din én siste gang med en anmodning om å slette alt lagret
under den, og glemmer så ID-en. Etter det knytter ingenting gjenværende rader
til deg, og en ny ID lages bare hvis du senere sier ja igjen.

## 6. Hvor lenge data lagres

| Data | Lagres |
|---|---|
| Alt på PC-en din | Til du sletter det. Spilletid og klipp beholdes til du fjerner dem; du kan slette alt fra appen |
| Statistikk: installasjonsrader | 24 måneder etter siste melding fra den installasjonen, så slettes de |
| Statistikk: daglig aktivitet og spillrader | 24 måneder |
| Statistikk: funksjonstellere per dag | På ubestemt tid — dette er summer uten ID og er ikke lenger personopplysninger |
| Serverlogger hos leverandøren | Kortvarig, som satt av leverandøren, for sikkerhet og feilsøking |

## 7. Hvem andre er involvert

Statistikktjenesten bruker to leverandører. Begge opptrer på opphavspersonens
instruks som databehandlere, og begge tilbyr databehandlervilkår som dekker
denne bruken:

- **Vercel Inc.** — er vert for nettsiden og endepunktet som mottar
  meldingene. Vercel er amerikansk med EU-regioner og infrastruktur.
- **Neon Inc.** — er vert for databasen der statistikken lagres. Databasen
  ligger i **EU (Frankfurt)**.

I tillegg er **GitHub, Inc.** vert for kildekoden og installasjonsfilen, og
betjener oppdateringssjekken. Når appen din ser etter oppdateringer, ser
GitHub forespørselen, inkludert IP-adressen din, under sin egen
personvernerklæring.

Ingen kjøper disse dataene, de brukes ikke til reklame, og de deles ikke med
andre. Opphavspersonen er den eneste som leser dashbordet, som ligger bak
passord.

## 8. Overføring utenfor EØS

Databasen ligger i EU. Vercel og GitHub er amerikanske selskaper, så
supportpersonell kan i prinsippet få tilgang til systemer utenfor EØS. Disse
overføringene bygger på Europakommisjonens **standard personvernbestemmelser
(SCC)** og, der leverandøren er sertifisert, **EU–US Data Privacy Framework**.

Dette er et rettsområde i bevegelse. Vil du helst ikke være en del av det i
det hele tatt, er det bare å la statistikken være av — da sendes ingenting om
deg noe sted.

## 9. Rettighetene dine

Etter GDPR kan du be om å:

- **se** hva som er lagret om deg (innsyn);
- **rette** noe som er feil;
- **slette** det;
- **begrense** hva som gjøres med det;
- **protestere** mot det;
- **få en kopi** i maskinlesbart format (dataportabilitet);
- **trekke tilbake samtykket** når som helst.

To av disse kan du utøve selv, med én gang, uten å spørre noen:
**Innstillinger → Brukervilkår** har **Eksporter dataene mine** (en JSON-fil
med alt GameHub har på PC-en din) og **Slett statistikken min**.

For alt annet, send e-post til **hakon.solvik@hotmail.com**. For å kunne svare
i det hele tatt trenger forespørselen installasjons-ID-en din — den står i
**Innstillinger → Brukervilkår**. Uten den finnes det ingen måte å finne
radene dine på, og ingen måte å vite at de er dine. Svar kommer innen 30
dager.

Du kan klage til **Datatilsynet** (datatilsynet.no) eller til
tilsynsmyndigheten i EØS-landet der du bor.

## 10. Barn

GameHub er for brukere fra 13 år og oppover. Brukere under 16 trenger
tillatelse fra en forelder eller verge før de slår på bruksstatistikk. Ingen
alder lagres, og det gjøres ingen forsøk på å identifisere barn — å spørre om
fødselsdato ville betydd å samle inn flere opplysninger enn appen ellers
trenger.

Er du forelder og tror barnet ditt har slått på statistikken, skriv til
adressen over, så blir den slettet.

## 11. Sikkerhet

- Statistikk-endepunktet godtar bare feltene som er listet over og avviser alt
  annet; meldingene går over HTTPS.
- Dashbordet ligger bak passord; øktinformasjonskapselen lagrer en hash, ikke
  passordet, og er httpOnly og merket secure.
- Databasen nås med legitimasjon som bare finnes i leverandørens
  miljøvariabler, aldri i kildekoden.
- Appen ber Windows om ingen tillatelse den ikke bruker, og har ingen
  fjernstyringskanal: ingen kan få din kopi av GameHub til å gjøre noe
  utenfra.

Finner du et sikkerhetsproblem, meld det til adressen over i stedet for å
publisere det. Se SECURITY.md i kodelageret.

## 12. Hvis noe går galt

Blir personopplysninger eksponert ved et uhell, vurderes hendelsen, og der det
sannsynligvis medfører risiko for noen, meldes den til Datatilsynet innen 72
timer etter at den ble oppdaget. Er risikoen for deg høy, får du beskjed
direkte i appen og på nettsiden.

## 13. Informasjonskapsler

Appen bruker ingen informasjonskapsler og ingen reklame- eller
sporingsteknologi. Nettsiden **webgamehubweb.vercel.app** setter én
informasjonskapsel, og bare for opphavspersonen: en innloggingskapsel for det
private dashbordet. Den er strengt nødvendig for den innloggingen og krever
ikke samtykke. Vanlige besøkende får ingen informasjonskapsler i det hele
tatt, og siden har ingen analyseverktøy.

## 14. Endringer

Denne erklæringen har versjonsnummer. Endres den på en måte som betyr noe,
sier appen fra neste gang du åpner den, og spør på nytt før noe nytt sendes.
Eldre versjoner beholdes i kodelageret så du kan se hva som er endret.

## 15. Kontakt

**Håkon Solvik**
E-post: hakon.solvik@hotmail.com
Personvernspørsmål, innsyn og sletting: samme adresse.
