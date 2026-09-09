# Lage GameHub.exe på din egen PC

Én fil å kjøre: **BUILD.bat**.

## Slik gjør du det

1. Pakk ut zip-en et sted, for eksempel `C:\GameHub`.
2. Dobbeltklikk **BUILD.bat**. (Eller: åpne Command Prompt, skriv `cd C:\GameHub`, trykk Enter, skriv `BUILD.bat`, trykk Enter.)
3. Vent. Første gang tar det noen minutter — Rust kompilerer alt én gang og
   husker det etterpå. Senere bygg tar under et minutt.
4. Når den er ferdig åpner den mappen **GameHub-ferdig** for deg.

Du får to filer:

| Fil | Hva det er |
|---|---|
| `GameHub-Setup.exe` | Kjør denne én gang. Installerer GameHub ordentlig, med snarvei i Start-menyen. **Denne er den du vil ha.** |
| `GameHub.exe` | Samme app uten å installere. Må bli liggende i mappen — `resources`-mappen ved siden av hører til den. |

Windows sier kanskje at utgiveren er ukjent. Det er fordi filen ikke er
kodesignert — velg **Mer informasjon** og så **Kjør likevel**.

## Hvis den stopper

BUILD.bat sjekker verktøyene før den starter, og sier hva som eventuelt mangler.
De to vanligste tingene:

**«Den Microsoft C++ linkeren ble ikke funnet»** — verktøyene er installert, men
ligger bare på PATH inne i sitt eget vindu. Åpne **x64 Native Tools Command
Prompt for VS** fra Start-menyen, skriv `cd C:\GameHub`, og kjør `BUILD.bat`
derfra.

**Bygget stopper midtveis** — lukk GameHub hvis den kjører. Windows lar ikke en
.exe som er i bruk bli erstattet. Hjelper ikke det, slett mappen
`apps\desktop\src-tauri\target` og prøv igjen.

## Å oppdatere senere

Denne utgaven oppdaterer seg ikke selv. Når du har endret koden: kjør BUILD.bat
på nytt og installer på nytt.

**Dataene dine blir stående.** Biblioteket, spillene du la til selv, coverbilder,
screenshots, hurtigtaster og spilletid ligger i din egen mappe utenfor
programmet, og GameHub tar en sikkerhetskopi automatisk før en ny versjon starter
første gang. Se **Innstillinger → Dine data**, eller `docs/YOUR-DATA.md`.

## Hvis du vil ha automatiske oppdateringer tilbake

Alt som trengs ligger fortsatt i prosjektet, bare avslått:

- `.github/workflows/release.yml` kjører ikke av seg selv lenger. Endre `on:` til
  å trigge på push igjen.
- `plugins.updater` er tatt ut av `apps/desktop/src-tauri/tauri.conf.json`. Legg
  seksjonen tilbake med endepunktet og din egen public key.
- `docs/RELEASING.md` beskriver de tre hemmelighetene i GitHub.
