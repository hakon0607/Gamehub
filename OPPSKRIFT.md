# Oppskrift: fra zip til ferdig GameHub.exe

Du trenger bare nettleseren. Ingen terminal, ingen kommandoer.

---

## Del 1 – Pakk ut zip-filen (2 minutter)

1. Finn `gamehubv16.zip` i **Nedlastinger**.
2. Høyreklikk på den → **Pakk ut alle** → **Pakk ut**.
3. Nå har du en mappe som heter `gamehubv16`. Åpne den. Inni ligger en mappe
   til som heter `gamehub`. Åpne den også. Du skal se filer som `README.md`,
   `package.json` og mapper som `apps` og `packages`.

## Del 2 – Gjør skjulte filer synlige (1 minutt) ⚠️ VIKTIG

Windows gjemmer noen filer som GitHub MÅ ha. Hvis du hopper over dette steget,
skjer ingenting på GitHub etterpå.

4. I mappevinduet (Utforsker), klikk på **Vis** øverst.
5. Klikk på **Vis** en gang til i menyen som kommer ned.
6. Klikk på **Skjulte elementer** så det får en hake.
7. Nå skal du se en mappe som heter `.github` og filer som heter `.gitignore`
   og `.npmrc`. Ser du dem? Bra. Ser du dem ikke? Gjør steg 4–6 igjen.

## Del 3 – Sjekk de tre hemmelighetene på GitHub (3 minutter)

Dette har du gjort før (i en tidligere versjon). Bare sjekk at de er der.

8. Gå til `github.com/hakon0607/gamehub` i nettleseren.
9. Klikk på **Settings** (tannhjulet, helt til høyre i menyen øverst).
10. I menyen til venstre: klikk **Secrets and variables** → **Actions**.
11. Du skal se tre navn i listen:
    - `TAURI_SIGNING_PRIVATE_KEY`
    - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
    - `TAURI_UPDATER_PUBKEY`

    Alle tre der? Gå til Del 4.

    Mangler noen? Klikk **New repository secret**, skriv navnet nøyaktig som
    over, og lim inn verdien:
    - `TAURI_SIGNING_PRIVATE_KEY`: åpne Notisblokk → **Fil → Åpne** → skriv
      `%USERPROFILE%\.tauri\gamehub.key` i feltet for filnavn → velg
      **Alle filer** nederst til høyre → **Åpne**. Trykk **Ctrl+A** og
      **Ctrl+C**. Lim inn med **Ctrl+V** på GitHub.
    - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: passordet du valgte da nøkkelen
      ble laget.
    - `TAURI_UPDATER_PUBKEY`: samme som første, men filen heter
      `gamehub.key.pub`.

12. Fortsatt i Settings: klikk **Actions** → **General** i menyen til venstre.
    Bla ned til **Workflow permissions**. Velg **Read and write permissions**
    og klikk **Save**. (Er det allerede valgt, gjør ingenting.)

## Del 4 – Last opp filene (5 minutter)

13. Gå tilbake til forsiden av repoet: klikk **Code** øverst.
14. Klikk den grønne knappen **Add file** → **Upload files**.
15. Gå til mappevinduet med `gamehub`-mappen (fra steg 3).
16. Klikk én gang inni mappen på et tomt sted, og trykk **Ctrl+A**. Alt blir
    markert – også `.github`.
17. Dra alt det markerte over til nettleseren og slipp det i den store
    stiplede boksen.
18. Vent til listen over filer er ferdig lastet. Bla gjennom listen: du skal
    se `.github/workflows/release.yml` et sted. Ser du den ikke? Gå tilbake
    til Del 2.
19. Bla helt ned. Klikk den grønne knappen **Commit changes**.

## Del 5 – Vent på at GitHub bygger .exe-filen (10–15 minutter)

20. Klikk på **Actions** øverst i repoet.
21. Du ser en rad som heter **Release** med en gul prikk som snurrer. Det betyr
    at GitHub bygger. Gå og hent deg noe å drikke.
22. Når prikken blir en grønn hake ✅ er den ferdig.

    Rød kryss ❌? Klikk på raden, så på det røde steget. Meldingen sier på
    vanlig norsk/engelsk hva som mangler (nesten alltid en av de tre
    hemmelighetene i Del 3).

## Del 6 – Hent .exe-filen (1 minutt)

23. Klikk **Code** øverst. På høyre side står det **Releases**. Klikk på
    **v1.0.0**.
24. Klikk på **GameHub-Setup.exe** for å laste ned.
25. Kjør filen. Windows sier kanskje «Windows beskyttet PC-en din» → klikk
    **Mer info** → **Kjør likevel**.

Ferdig! 🎉

---

## Neste gang du skal gi ut en ny versjon

Du trenger ikke laste opp noe som helst. Bare:

1. Gå til `github.com/hakon0607/gamehub`.
2. Klikk deg inn i `apps` → `desktop` → `src-tauri` → `tauri.conf.json`.
3. Klikk på blyanten ✏️ (øverst til høyre over filen).
4. Finn linjen `"version": "1.0.0"`. Endre `1.0.0` til `1.0.1`.
5. Klikk den grønne knappen **Commit changes** (to ganger).
6. Vent 10–15 minutter (Del 5). Alle som har GameHub installert får nå en
   popup i appen som sier «GameHub 1.0.1 er klar» med knappen **Oppdater nå**.

Hvis du senere laster opp en ny zip fra meg: gjør Del 1, 2 og 4 igjen. Tallet
i `tauri.conf.json` i zip-en er alltid høyere enn forrige, så releasen starter
av seg selv.

---

## Første gang du bruker de nye funksjonene

- **Lyd i replay:** ingenting å gjøre. Det er på. Sjekk at det står
  «🔊 Spillyd fra …» på Replay-siden når replay er på.
- **Lengre replay:** Replay-siden → dra glidebryteren «Hvor mye som huskes».
- **Frys spillet:** start et spill → trykk **F7** midt i en cutscene → spillet
  står stille. Trykk **F7** igjen for å fortsette. Vil du at frysen også skal
  ta kopi av lagringen: åpne spillet i GameHub → **Frys og lagring** →
  **Finn** → velg riktig mappe. Gjøres én gang per spill.
- **Finn en innstilling:** trykk **Ctrl+K** og skriv hva du leter etter.
