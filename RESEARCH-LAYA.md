# Research — kan Laya styra våra spelare?

**Datum:** 2026-09-21
**Branch:** `claude/laya-player-control-aztb3r`
**Fråga:** Kan [`convaiinnovations/laya`](https://huggingface.co/convaiinnovations/laya) ersätta eller komplettera
våra 146 handtränade parametrar per lag?

---

## Slutsats först

| Nivå | Frekvens | Verdikt |
|------|----------|---------|
| **A. Spelarbeslut** (`v6_tick`) | 60 Hz × 10 spelare | ❌ **Omöjligt** — ~86 000× för långsamt |
| **B. Coach-direktiv** (`CoachDirective`) | 0,2 Hz × 2 lag | ⚠️ Möjligt live, **omöjligt under träning**, blockeras av browsern |
| **C. Offline coach-analys** → `coach_nudge.py` | ~1/träningssession | ✅ **Här finns den verkliga möjligheten** |

Laya är inte en kontrollpolicy. Det är en *textklassificerare med kalibrerade sannolikheter*.
Den hör hemma i tränarrummet, inte på planen.

---

## 1. Vad Laya faktiskt är

Hämtat från modellkort, GitHub-portarna och PyPI (HF var blockerat i sandlådan, se §6).

| Egenskap | Värde |
|---|---|
| Arkitektur | **ModernBERT-large encoder, 421M parametrar** (multilingual: mmBERT-base, 322M) |
| Typ | Non-autoregressiv — **genererar ingen text**, en forward pass |
| Kontextfönster | 512 tokens (eng.) / 1024 (multilingual) |
| Indata | Fritext, e-post, ticket eller **JSON** + typade frågor |
| Utdata | 3 primitiv: `choice` (fördelning över namngivna alternativ), `score` (förväntad nivå på ordinal rubrik), `bool` (P(sant)) — alla med kalibrerad sannolikhet |
| Träning | RLCD — REINFORCE med group-mean baseline (GRPO-liknande), belöning = strikt proper scoring rule |
| Licens | Apache 2.0, öppna vikter |
| Paket | `pip install laya` (0.3.4) → kräver `torch`, `transformers`, `safetensors` |

### Latens (avgörande för oss)

| Miljö | Latens |
|---|---|
| Tesla T4 GPU, 1 fråga | **32,8–39,5 ms** |
| Tesla T4, batch 50 | 337 ms (6,8 ms/fråga, 103–332 q/s) |
| **CPU (generisk)** | **193–464 ms per request** |
| ONNX-port på Apple silicon | ~140 ms |
| MLX på M3 Max | 13,4 ms (kort fråga), 146,8 q/s i batch om 50 |

### Diskfotavtryck

**~1,7 GB fp32** (ONNX-porten), ~2 GB RAM laddat. Det är den siffran som dödar browser-spåret.

### Känd begränsning

`head_max_len` är 192–256 tokens för alternativbeskrivningar. **Över ~20 alternativ degraderar
träffsäkerheten kraftigt** (vid 77 alternativ: 0,425 accuracy mot konkurrenters 0,870).
Vår parameterrymd har 146 dimensioner — den får aldrig presenteras som ett platt val.

---

## 2. Vad vår motor faktiskt kräver

Mätt på denna maskin (4× Xeon @ 2,10 GHz) med
[`training-engine/examples/bench_ticks.rs`](training-engine/examples/bench_ticks.rs):

```
ticks/match            : 9000          (150 s × 60 fps)
spelarbeslut/match     : 89 384
tid 1 match (1 tråd)   : 34,46 ms
seriellt               : 29 matcher/s
parallellt (4 kärnor)  : 111 matcher/s
=> spelarbeslut/s      : 9,9 × 10⁶
```

Per beslut: **~385 ns** — och det inkluderar fysik, spatial-sökning och regelmotor.

---

## 3. Gap-analys

### A. Spelarbeslut — 60 Hz

```
89 384 beslut/match × 33 ms (GPU, bästa fall)  =  2 950 s  =  49 minuter per match
89 384 beslut/match × 385 ns (idag)            =  34 ms
```

**Faktor ~86 000×.** Även med perfekt batchning (332 q/s på T4) blir det 4,5 minuter per match
— fortfarande ~7 800× långsammare än idag.

En träningssession kör tusentals matcher parallellt. `--quick` motsvarar ~375 000 evalueringar.
Det blir i storleksordningen 10¹⁰ forward passes. Spåret är inte "dyrt", det är stängt.

Utöver hastigheten finns tre strukturella problem:

1. **Utdata matchar inte.** Vi behöver 146 kontinuerliga floats (avstånd i pixlar, sannolikheter,
   trösklar). Laya ger kategorifördelningar. `score` ger visserligen en kontinuerlig förväntad nivå,
   men på en ordinal skala med ett fåtal steg — inte `ownGoal.preferred = 287,4`.
2. **Indata matchar inte.** Tillståndet skulle behöva serialiseras till text/JSON 89 384 gånger per
   match och tokeniseras. Tokeniseringen ensam är dyrare än hela vår nuvarande tick.
3. **Träningsloopen matchar inte.** Vår trainer är (1+1)-evolutionär med statistisk early-stop
   (`trainer.rs`, z-score ±2,5). Laya är en inferensmodell tränad med RLCD på GPU. Det finns
   ingen gradient att koppla in i vår mutation, och inget sätt att mutera 421M vikter mot måldiff.

### B. Coach-direktiv — 0,2 Hz

Detta är den *enda* platsen i realtidsslingan där kadensen är rimlig.
`V7Team::pre_tick` uppdaterar `CoachDirective` var 300:e tick:

```
9000 / 300 = 30 uppdateringar per match och lag  →  60 anrop/match
```

Och `score`-primitivet passar faktiskt formen: `CoachDirective` är fyra floats i [0,1]
(`press_intensity`, `line_height`, `compactness`, `tempo`) — det är fyra ordinala rubriker.
`GlobalBehavior` (`detector.rs`) ger redan exakt de features som skulle serialiseras:
`opp_avg_x`, `opp_press_rate`, `space_behind`.

Men tre blockerare kvarstår:

1. **Träning.** 60 anrop × 2 000 matcher/epok = 120 000 forward passes per epok. Vid 332 q/s (T4)
   = 6 minuter Laya-tid per epok, mot 18 sekunder för hela epoken idag. ~20× nedgång — på en GPU
   vi inte har. På CPU: timmar per epok.
2. **Browsern.** Spelet kör Rust-motorn som WASM (`match-engine-wasm/`, se `HANDOFF-WASM.md`).
   1,7 GB modell går inte att skicka till en webbläsare. Int8-kvantisering ger ~420 MB — fortfarande
   uteslutet, och ingen kvantiserad variant är officiellt publicerad.
3. **Determinism.** Vår trainer bygger på seedade `SmallRng` och reproducerbara matcher.
   En extern modell i slingan bryter reproducerbarheten om den inte är bitidentisk mellan körningar.

Om man ändå vill prova: kör Laya **utanför** träningen, som en offline-generator som producerar en
uppslagstabell `GlobalBehavior → CoachDirective`, och baka in tabellen i motorn. Då betalar man
inferenskostnaden en gång. Men då har man egentligen bara byggt en dyr lookup-table — och
`compute_directive()` är redan den tabellen, tränad mot faktiska matchresultat.

### C. Offline coach-analys — ~1 per session ✅

Här är formen en exakt träff. `coach_nudge.py` löser redan idag ett typat beslutsproblem:

> Givet lagets historik och matchstatistik — vilken parameter ska knuffas, åt vilket håll?

Det är bokstavligen ett `choice`-problem över ~18 namngivna parametrar (`PARAM_BOUNDS`),
plus ett `score` för riktning och magnitud. Och:

- **Latens är irrelevant** — några dussin frågor per träningssession.
- **Indata är redan JSON** — `EvalResult` från `trainer.rs` bär `avg_passes`,
  `pass_completion_rate`, `avg_shots`, `tackle_success_rate`, `point_z_score` m.m.
- **Kalibrerade sannolikheter är faktiskt värdefulla här.** `coach_nudge.py` tillåter en nudge per
  100 000 evalueringar. Med ett budgetsystem så snävt vill man veta *hur säker* rekommendationen är,
  inte bara vad den är. Det är precis vad RLCD-träningen köper oss.
- **18 alternativ ligger precis under `head_max_len`-taket** på ~20.
- **Flerspråkighet spelar roll här** — `coach.md` och `coaching.md` är skrivna på svenska, och
  `laya-multilingual` täcker 100+ språk. Tränarpersonan kan matas in som den är.

Skiss:

```python
questions = {
    "problem": {
        "type": "choice",
        "instructions": "Vad är lagets främsta svaghet enligt matchstatistiken?",
        "criteria": {
            "passning": "låg passningsprocent, bollförluster i uppbyggnad",
            "avslut": "många skott men få mål, dålig avslutsposition",
            "press": "motståndaren får fritt utrymme, låg tackling",
            "linje": "insläppta mål på djupled, hög backlinje utnyttjas",
        },
    },
    "sakerhet": {
        "type": "score",
        "instructions": "Hur starkt stödjer statistiken slutsatsen?",
        "criteria": ["svagt underlag", "indikativt", "tydligt mönster"],
    },
}
```

Utdatans sannolikhet gate:ar sedan om nudgen alls ska göras — t.ex. kräv P > 0,7 innan
`coach_nudge.py nudge` körs. Det ersätter inget i motorn; det gör tränarjournalen datadriven.

---

## 4. Alternativet om målet är rikare spelarstyrning

Om den verkliga drivkraften är "146 parametrar är trubbigt, jag vill ha något som *reagerar på
situationen*" — då är svaret ett litet nätverk i Rust, inte en språkmodell.

Mätt med [`training-engine/examples/bench_mlp.rs`](training-engine/examples/bench_mlp.rs):

| Nät | Vikter | ns/beslut | Beslut/s | Kostnad mot idag |
|---|---|---|---|---|
| 20-8-13 | 285 | 189 | 5,3M | +49 % |
| **24-16-16** | **672** | **327** | **3,1M** | **+85 %** |
| 32-32-16 | 1 584 | 667 | 1,5M | +173 % |
| 48-64-20 | 4 436 | 1 534 | 0,65M | +298 % |
| 64-128-24 | 11 416 | 4 648 | 0,22M | +1 107 % |

*(Referens: nuvarande `v6_tick` ≈ 385 ns/beslut inklusive fysik.)*

Ett **24-16-16-nät (672 vikter)** nästan dubblar kostnaden per beslut — träningen går från
111 till ~60 matcher/s. Fullt hanterbart. Och till skillnad från Laya:

- Indata finns redan färdig: `PlayerContext` (5 features) + `GlobalBehavior` (3) + bollgeometri ≈ 20 inputs.
- Utdata är kontinuerlig och i rätt form — nätet kan modulera `V6Params` precis som
  `apply_directive()` redan gör, men situationsberoende istället för via fyra globala reglage.
- Det kompilerar till WASM och körs i browsern utan extra beroenden.
- Det tränas med **samma** evolutionära loop.

Den sista punkten har en varning: 672 dimensioner mot dagens 146 är 4,6× större sökrymd.
(1+1)-ES blir trögt där. Man vill sannolikt byta `trainer.rs` mutationsstrategi till
populationsbaserad ES (OpenAI-ES / CMA-ES-liknande) innan man skalar upp parameterantalet.
Det är den egentliga uppgiften — inte valet av modellarkitektur.

---

## 5. Rekommendation

1. **Släpp Laya för spelarstyrning.** Fel verktygsklass, fel storleksordning.
2. **Överväg Laya i `coach_nudge.py`** som ett kalibrerat beslutsstöd för vilken parameter som ska
   knuffas. Låg risk, kör offline, rör inte motorn, Apache 2.0. Detta är den enda integration som
   bär sin egen vikt.
3. **Om målet är rikare spelar-AI:** prototypa ett 24-16-16-nät som moduleringslager ovanpå
   `V6Params`, och uppgradera `trainer.rs` till populationsbaserad ES först.

---

## 6. Reproducerbarhet & källor

Mätningarna kan köras om:

```bash
cd training-engine
cargo run --release --example bench_ticks   # motorns genomströmning
cargo run --release --example bench_mlp     # MLP-kostnad per beslut
```

`huggingface.co` och `laya.convaiinnovations.com` är blockerade av sandlådans egress-proxy, så
modellkortet lästes inte direkt. Specifikationerna nedan kommer från GitHub-porten och PyPI,
som båda var åtkomliga. **Verifiera parameterantal och latens mot modellkortet innan beslut.**

- [convaiinnovations/laya](https://huggingface.co/convaiinnovations/laya) — modellkort
- [NandhaKishorM/laya](https://github.com/NandhaKishorM/laya) — referensimplementation, arkitektur & benchmarks
- [receptron/laya](https://github.com/receptron/laya) — ONNX-port, diskstorlek & CPU-latens
- [mizorewww/laya-mlx](https://github.com/mizorewww/laya-mlx) — MLX-port, Apple silicon-latens
- [laya på PyPI](https://pypi.org/project/laya/) — beroenden & licens
- [laya.convaiinnovations.com](https://laya.convaiinnovations.com/) — produktsida
