# Sette opp nettsiden

Tre ting, alt i nettleseren. Regn med ti minutter.

## 1. Legg koden på GitHub

Lag et nytt, tomt repo som heter `gamehub-web`. Pakk ut denne zip-en, slå på
**View → Show → Hidden items** i Utforsker (ellers blir `.gitignore` liggende
igjen), marker alt og dra det inn i GitHub.

## 2. Koble det til Vercel

På [vercel.com](https://vercel.com): **Add New → Project**, velg `gamehub-web`,
og trykk **Deploy**. Vercel kjenner igjen Next.js selv — du skal ikke endre noen
innstillinger.

Siden er oppe etter et minutt, men uten fillagring ennå.

## 3. Lag Blob-lageret

I prosjektet på Vercel: **Storage → Create Database → Blob → Create**.

Det er alt. Vercel legger inn `BLOB_READ_WRITE_TOKEN` som miljøvariabel selv, og
den er den eneste nøkkelen siden trenger. Gå til **Deployments** og trykk
**Redeploy** på den øverste, så plukker siden den opp.

## 4. Bytt passordet

Under **Settings → Environment Variables**, legg til:

| Name | Value |
|---|---|
| `ADMIN_USERNAME` | det du vil |
| `ADMIN_PASSWORD` | noe langt og tilfeldig |
| `AUTH_SECRET` | en lang tilfeldig streng |

Så **Redeploy** igjen.

Dette står som punkt fire, men det er det viktigste på siden. `/admin` er en
adresse hvem som helst kan gjette, og `admin123` er det første passordet noen
prøver. Den som kommer inn kan legge ut en hvilken som helst .exe under ditt
navn, som folk laster ned og kjører på maskinen sin. Admin-siden viser en gul
advarsel så lenge standardpassordet står — den forsvinner når du har byttet.

Lar du det stå mens du tester alene er det greit. Deler du lenken med noen, bytt
det først.

## Legge ut en versjon

1. Gå til `/admin` og logg inn.
2. Fyll inn versjonsnummer (`0.3.0`), gjerne et navn, og hva som er nytt.
3. Velg `GameHub-Setup.exe` fra `GameHub-ferdig`-mappa.
4. **Publiser.**

Filen går rett fra nettleseren din til lageret — ikke via serveren, som ville
stoppet på 4,5 MB. Store filer tar litt tid; framdriften vises underveis.

Versjonen dukker opp på forsiden, på `/last-ned` og under `/versjoner` med én
gang. Laster du opp flere filer til samme versjonsnummer, legges de til der i
stedet for å lage en ny rad — slik får du både installer og portabel utgave på
samme versjon.

## Hva det koster

Ingenting, innenfor Vercels gratisnivå: 5 GB lagring og 100 GB nedlasting i
måneden. Med en installer på rundt 90 MB er det omtrent tusen nedlastinger i
måneden.

Verdt å vite: går du over grensen slutter Blob å virke i 30 dager i stedet for å
sende deg en regning. Nærmer du deg, sender Vercel e-post. Skjer det, si fra —
da flytter vi filene til GitHub Releases, som ikke har noen grense på nedlasting.

## Sidene

| Adresse | Hva det er |
|---|---|
| `/` | Forsiden: hva appen er, og knapp til nyeste versjon |
| `/last-ned` | Nedlastingssiden med den store knappen |
| `/versjoner` | Alle versjoner med navn, beskrivelse og dato |
| `/admin` | Innlogging og opplasting |

## Hvis noe ikke virker

**«Fillagringen er ikke satt opp»** på admin-siden — Blob-lageret mangler, eller
du har ikke redeployet etter at du lagde det.

**Opplastingen stopper med «Du er ikke logget inn»** — økten varer åtte timer.
Last siden på nytt og logg inn igjen.

**Ingenting vises på forsiden** — ingen versjon er publisert ennå, eller den som
er publisert har ingen fil.
