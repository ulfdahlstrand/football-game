# Fotbolls-RPG

Ett litet fotbolls-RPG byggt med React och Babel direkt i webbläsaren. Projektet har inget byggsteg och kräver ingen installation för att köras lokalt.

## Starta spelet lokalt

Spelet laddar `.jsx`-filer via `<script type="text/babel">`, så kör det via en lokal webbserver istället för att dubbelklicka på HTML-filen.

### Python

```bash
cd /Users/ulfdahlstrand/Projects/Code/Private/Spel/football-game
python3 -m http.server 8000
```

Öppna sedan:

```text
http://localhost:8000/Fotbolls-RPG.html
```

### Node / npx

```bash
cd /Users/ulfdahlstrand/Projects/Code/Private/Spel/football-game
npx serve .
```

Öppna den URL som `serve` skriver ut, till exempel:

```text
http://localhost:3000/Fotbolls-RPG.html
```

## Kontroller i matchen

| Tangent | Handling |
| --- | --- |
| `Pilar` / `A` `S` `D` | Rörelse |
| `W` | Passning |
| `W` + piltangent | Riktad passning |
| `Space` | Skott |
| `Q` + `Space` | Superskott |
| `E` | Tackling |
| `Enter` | Hopp |
| `Esc` | Lämna matchen |

Vid straff: håll piltangent för riktning och tryck `Space` för att skjuta.

## Projektstruktur

```text
Fotbolls-RPG.html     Startfilen för spelet
app.jsx               Rotnivå, spellägen och övergångar
game-world.jsx        Öppen värld, NPC-rörelser och skyltar
football-match.jsx    5 mot 5-match, AI, tacklingar och fasta situationer
match-engine.js       Ren JS-matchmotor for Node/headless-simuleringar
simulate-match.js     Script som kor matchmotorn utan browser
data/policies/        Sparade AI-policyfiler/parametrar
match-screen.jsx      Match-UI och äldre matchskärmsdelar
questions.jsx         Quizfrågor
binder.jsx            Delade hjälpfunktioner
tweaks-panel.jsx      Utvecklarpanel
```

## Headless matchmotor

Det finns en ren JavaScript-motor i `match-engine.js` som kan koras i Node utan React, DOM eller canvas. Den ar for simulering, statistik och framtida AI-traning.

Kor en snabb simulering:

```bash
node simulate-match.js 20
```

Jamfor framtida ny AI mot fryst baseline:

```bash
node simulate-match.js 100 candidate-vs-baseline
```

Tillgangliga lagen i scriptet:

```text
baseline-vs-baseline
candidate-vs-baseline
baseline-vs-candidate
```

Just nu ar `candidate` medvetet samma beteende som `baseline`. Nar traningen ar pa plats kan den nya policyn bytas in dar och testas mot den gamla AI:n over manga matcher.

AI-policyfiler sparas i:

```text
data/policies/
```

Just nu finns:

```text
data/policies/baseline.json
data/policies/candidate.json
```

Nar vi borjar trana kan scriptet skriva nya parametrar eller vikter till `candidate.json` eller skapa filer som `candidate-001.json`, `best.json` och liknande. Tanken ar att bade Node-simuleringen och browser-spelet ska kunna lasa samma policyfil.

I den spelbara browser-matchen laddar motståndarlaget `data/policies/candidate.json`. Om filen inte kan laddas faller spelet tillbaka till baseline-AI.

## Deploy online gratis

Det här projektet är en statisk sida, så det passar bra på gratis statisk hosting.

Snabbaste alternativen:

1. **Netlify Drop**  
   Dra hela projektmappen till Netlify Drop. Det är snabbast om du bara vill få en spelbar länk direkt.

2. **Cloudflare Pages**  
   Bra gratisval för statiska projekt. Koppla ett GitHub-repo eller ladda upp filerna direkt. Cloudflare Pages ger en gratis `*.pages.dev`-adress.

3. **GitHub Pages**  
   Bra om projektet ändå ligger på GitHub. Lägg filerna i ett repo och aktivera Pages i repo-inställningarna.

4. **Vercel**  
   Funkar också för statiska projekt på Hobby-planen, särskilt om du vill koppla GitHub och få auto-deploy.

### Viktigt för deploy

De flesta hostar letar efter en fil som heter `index.html`. Projektet har därför en liten `index.html` som skickar vidare till spelets startfil `Fotbolls-RPG.html`.

Spelet fungerar också direkt via:

```text
https://din-sida.example/Fotbolls-RPG.html
```

## Rekommendation

För snabbast test online: använd **Netlify Drop**.

För bäst långsiktig gratisdeploy: lägg projektet på GitHub och använd **Cloudflare Pages** eller **GitHub Pages**.
