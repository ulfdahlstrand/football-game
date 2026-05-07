// Football RPG — 2.5D isometric prototype
const { useState, useEffect, useRef, useCallback, useMemo } = React;

// ——————————————————————————————————————————————
// DATA
// ——————————————————————————————————————————————

const PITCHES = [
  { id: "p1", name: "Parkängen",       x: 3,  y: 3,  difficulty: 1, color: "#4a9d54" },
  { id: "p2", name: "Grusplan Öst",    x: 11, y: 5,  difficulty: 2, color: "#6b8a3a" },
  { id: "p3", name: "Skolplanen",      x: 6,  y: 10, difficulty: 2, color: "#4a9d54" },
  { id: "p4", name: "Sjökantens IP",   x: 14, y: 11, difficulty: 3, color: "#3d8a47" },
  { id: "p5", name: "Bergsvallen",     x: 4,  y: 15, difficulty: 3, color: "#5a9a52" },
  { id: "p6", name: "Centralarenan",   x: 12, y: 17, difficulty: 4, color: "#2f7d3a" },
];

// Questions pool — original scenarios, not tied to any league or real clubs.
// Difficulty 1 (basic rules) → 4 (tactical/ethics)
const QUESTIONS = {
  1: [
    {
      q: "Bollen rullar ut över sidlinjen efter att motståndaren sparkat den. Vad händer?",
      options: ["Hörna", "Inkast åt ditt lag", "Frispark åt ditt lag"],
      answer: 1,
      why: "När bollen går över sidlinjen blir det inkast. Det laget som INTE sparkade bollen ut får kasta in den."
    },
    {
      q: "Du vill tacka din lagkamrat efter ett fint pass. Vad är ett bra sätt?",
      options: ["Hålla tummen upp och ropa bra!", "Ignorera och springa vidare", "Klaga att passet var för hårt"],
      answer: 0,
      why: "Positiv feedback bygger lagkänsla. Uppmuntran gör att alla vågar ta initiativ och våga göra misstag."
    },
    {
      q: "Domaren blåser för frispark mot ert lag. Vad gör du?",
      options: ["Protesterar högljutt", "Accepterar beslutet och sätter upp mur", "Tar bollen och springer iväg"],
      answer: 1,
      why: "Respekt för domaren är en grundregel. Protester leder till gult kort och stör lagets fokus."
    },
    {
      q: "Du är längst bak i försvaret och får bollen. En motståndare pressar hårt. Vad är säkrast?",
      options: ["Dribbla förbi in i eget straffområde", "Spela tillbaka till målvakten", "Skjuta iväg den i sidled utan att titta"],
      answer: 1,
      why: "En kontrollerad passning till målvakten är säker. Att panikklarera kan ge bollen direkt till motståndaren."
    },
  ],
  2: [
    {
      q: "Motståndaren fäller dig precis utanför straffområdet. Du ligger kvar. Vad händer troligen?",
      options: ["Straff", "Frispark", "Hörna"],
      answer: 1,
      why: "Utanför straffområdet = frispark. Innanför = straff. Linjen spelar roll, även centimeter räknas."
    },
    {
      q: "Ni leder 1-0 med 10 minuter kvar. Vad är en smart taktik?",
      options: ["Alla går upp för att göra fler mål", "Hålla bollen, spela lugnt, täppa till mittfältet", "Sparka bort bollen varje gång"],
      answer: 1,
      why: "Bollinnehav i slutet av en match tar tid från motståndaren. Panikklareringar ger bollen tillbaka snabbt."
    },
    {
      q: "Din lagkamrat missar ett öppet mål. Hur reagerar du?",
      options: ["Skäller ut hen direkt", "Peppar och säger 'nästa gång!'", "Slutar passa till hen"],
      answer: 1,
      why: "Alla missar. En lagkamrat som får stöd efter en miss vågar försöka igen. Kritik i stunden sänker prestationen."
    },
    {
      q: "Offside — när gäller regeln?",
      options: ["När du är framför bollen vid passmomentet", "När du springer snabbare än försvararen", "Vid inkast och hörna"],
      answer: 0,
      why: "Offside = du är närmare motståndarnas mållinje än både bollen OCH näst sista försvararen när bollen spelas. Offside gäller inte vid inkast eller hörna."
    },
  ],
  3: [
    {
      q: "Ni spelar 4-3-3. Din ytter är täckt. Var bör du som mittfältare söka bollen?",
      options: ["Bredvid din ytter", "Mellan linjerna, bakom motståndarens mittfält", "Vid egen målvakt"],
      answer: 1,
      why: "Att söka bollen mellan linjerna skapar övertal och tvingar försvaret att bryta sin form. En grundpelare i positionsspel."
    },
    {
      q: "Motståndaren har boll och ni pressar högt. Vad är viktigast?",
      options: ["Alla springer mot bollhållaren", "Stänga passningsvägar, pressa tillsammans", "Backa hem snabbt"],
      answer: 1,
      why: "Högt press fungerar bara om passningsalternativen också täcks. Annars spelar motståndaren sig enkelt förbi."
    },
    {
      q: "Du har bollen och ser en löpning. Ska du passa eller dribbla?",
      options: ["Alltid dribbla själv", "Passa om löpningen är fri och skapar fara", "Alltid slå långt"],
      answer: 1,
      why: "Det bästa valet är situationsberoende. En fri löpare i bra position är nästan alltid ett bättre val än att hålla bollen själv."
    },
    {
      q: "Domaren dömer fel. Du såg det tydligt. Vad är rätt?",
      options: ["Skrika åt domaren", "Prata lugnt med kaptenen, låt kaptenen prata med domaren", "Ta saken i egna händer"],
      answer: 1,
      why: "Bara kaptenen bör tala med domaren. Det visar respekt och ger bäst chans att bli lyssnad på."
    },
  ],
  4: [
    {
      q: "Det är slutminuten. Ni behöver mål. Målvakten vill gå upp på hörna. Vad är risken?",
      options: ["Ingen risk — alltid bra", "Tomt mål om motståndaren bryter och slår långt", "Domaren blåser av"],
      answer: 1,
      why: "När målvakten går upp är buren tom. Det är en avvägning: chans till mål vs. risk för baklängesmål på långt avslut."
    },
    {
      q: "En motståndare är skadad. Ni har bollen i anfall. Vad är fair play?",
      options: ["Fortsätt anfallet till mål", "Sparka ut bollen så hen kan få vård", "Låtsas inte se"],
      answer: 1,
      why: "Fair play går före resultatet. Traditionen är att sparka ut bollen vid tydliga skador, och motståndaren ger tillbaka bollen efter avbrottet."
    },
    {
      q: "Ni spelar med låg blockordning (försvarar djupt). Vad är nyckeln för att det ska fungera?",
      options: ["Alla springer runt fritt", "Kompakta led, korta avstånd mellan spelare", "Ingen kommunikation"],
      answer: 1,
      why: "Ett lågt block kräver att leden är tight ihop. Stora avstånd = stora luckor för motståndaren att spela genom."
    },
    {
      q: "Du blir utbytt i 60:e minuten. Du tycker det är orättvist. Vad gör du?",
      options: ["Vägrar lämna planen", "Går av, sätter dig, stöttar laget resten av matchen", "Kastar ditt linnet på tränaren"],
      answer: 1,
      why: "Ett byte är tränarens beslut. Proffsiga spelare hanterar besvikelse genom att fortsätta stötta laget — det märks och påverkar framtida val."
    },
  ],
};

// ——————————————————————————————————————————————
// TWEAK DEFAULTS
// ——————————————————————————————————————————————
const TWEAK_DEFAULTS = /*EDITMODE-BEGIN*/{
  "playerSpeed": 0.12,
  "mapTheme": "day",
  "showGrid": false,
  "fastIntro": false
}/*EDITMODE-END*/;

// ——————————————————————————————————————————————
// ISO HELPERS
// ——————————————————————————————————————————————
const TILE_W = 64;
const TILE_H = 32;

function isoToScreen(x, y) {
  return {
    sx: (x - y) * (TILE_W / 2),
    sy: (x + y) * (TILE_H / 2),
  };
}

// ——————————————————————————————————————————————
// MAIN APP
// ——————————————————————————————————————————————
function App() {
  const [tweaks, setTweaks] = useTweaks(TWEAK_DEFAULTS);
  const [view, setView] = useState("map"); // map | match | penalty | result | binder
  const [player, setPlayer] = useState({ x: 7, y: 7 });
  const [activePitch, setActivePitch] = useState(null);
  const [achievements, setAchievements] = useState([]); // array of pitch ids
  const [matchState, setMatchState] = useState(null);
  const [penaltyState, setPenaltyState] = useState(null);
  const [resultState, setResultState] = useState(null);
  const [toast, setToast] = useState(null);

  // ——— movement ———
  const keys = useRef({});
  useEffect(() => {
    const down = e => { keys.current[e.key.toLowerCase()] = true; };
    const up   = e => { keys.current[e.key.toLowerCase()] = false; };
    window.addEventListener("keydown", down);
    window.addEventListener("keyup", up);
    return () => {
      window.removeEventListener("keydown", down);
      window.removeEventListener("keyup", up);
    };
  }, []);

  useEffect(() => {
    if (view !== "map") return;
    let raf;
    const tick = () => {
      setPlayer(p => {
        let { x, y } = p;
        const s = tweaks.playerSpeed;
        if (keys.current["w"] || keys.current["arrowup"])    y -= s;
        if (keys.current["s"] || keys.current["arrowdown"])  y += s;
        if (keys.current["a"] || keys.current["arrowleft"])  x -= s;
        if (keys.current["d"] || keys.current["arrowright"]) x += s;
        x = Math.max(0.5, Math.min(18.5, x));
        y = Math.max(0.5, Math.min(19.5, y));
        return { x, y };
      });
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [view, tweaks.playerSpeed]);

  // ——— pitch proximity ———
  const nearPitch = useMemo(() => {
    for (const p of PITCHES) {
      const dx = p.x - player.x, dy = p.y - player.y;
      if (dx*dx + dy*dy < 1.3*1.3) return p;
    }
    return null;
  }, [player]);

  // Enter pitch on E
  useEffect(() => {
    if (view !== "map") return;
    const onKey = (e) => {
      if (e.key.toLowerCase() === "e" && nearPitch) {
        startMatch(nearPitch);
      }
      if (e.key.toLowerCase() === "b") {
        setView("binder");
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [nearPitch, view]);

  const startMatch = (pitch) => {
    setActivePitch(pitch);
    const qs = [...QUESTIONS[pitch.difficulty]].slice(0, 4);
    setMatchState({
      pitch,
      questions: qs,
      idx: 0,
      correct: 0,
      wrong: 0,
      history: [], // {q, picked, correct}
    });
    setView("match");
  };

  const answerQuestion = (pickedIdx) => {
    setMatchState(ms => {
      const q = ms.questions[ms.idx];
      const isRight = pickedIdx === q.answer;
      const history = [...ms.history, { q, picked: pickedIdx, correct: isRight }];
      const correct = ms.correct + (isRight ? 1 : 0);
      const wrong = ms.wrong + (isRight ? 0 : 1);
      const nextIdx = ms.idx + 1;

      if (nextIdx >= 4) {
        // finish match
        setTimeout(() => finishMatch(correct, wrong, history), 900);
      }
      return { ...ms, idx: nextIdx, correct, wrong, history };
    });
  };

  const finishMatch = (correct, wrong, history) => {
    if (correct >= 3) {
      awardMatch(activePitch, history, false);
    } else if (correct === 2 && wrong === 2) {
      // penalty shoot
      setPenaltyState({ attempts: 0, scored: 0, log: [], done: false });
      setView("penalty");
    } else {
      setResultState({ won: false, pitch: activePitch, history, reason: "För få rätt — träna mer och försök igen!" });
      setView("result");
    }
  };

  const awardMatch = (pitch, history, viaPenalty) => {
    setAchievements(a => a.includes(pitch.id) ? a : [...a, pitch.id]);
    setResultState({ won: true, pitch, history, viaPenalty });
    setView("result");
  };

  const shootPenalty = (dir) => {
    // dir: "left" | "center" | "right"
    const keeperGoes = ["left", "center", "right"][Math.floor(Math.random() * 3)];
    const scored = keeperGoes !== dir;
    setPenaltyState(ps => {
      const attempts = ps.attempts + 1;
      const log = [...ps.log, { dir, keeper: keeperGoes, scored }];
      const scoredTotal = ps.scored + (scored ? 1 : 0);
      const done = attempts >= 3;
      if (done) {
        setTimeout(() => {
          if (scoredTotal >= 2) awardMatch(activePitch, matchState.history, true);
          else setResultState({ won: false, pitch: activePitch, history: matchState.history, reason: "Straffarna satt inte — bättre lycka nästa gång!", viaPenalty: true });
          setView(scoredTotal >= 2 ? "result" : "result");
        }, 1200);
      }
      return { attempts, scored: scoredTotal, log, done };
    });
  };

  const backToMap = () => {
    setView("map");
    setActivePitch(null);
    setMatchState(null);
    setPenaltyState(null);
    setResultState(null);
  };

  // ——— RENDER ———
  return (
    <div className="app" data-theme={tweaks.mapTheme}>
      {view === "map"     && <MapView player={player} pitches={PITCHES} achievements={achievements} nearPitch={nearPitch} showGrid={tweaks.showGrid} onOpenBinder={() => setView("binder")} />}
      {view === "match"   && <MatchView state={matchState} onAnswer={answerQuestion} />}
      {view === "penalty" && <PenaltyView state={penaltyState} pitch={activePitch} onShoot={shootPenalty} />}
      {view === "result"  && <ResultView state={resultState} onBack={backToMap} />}
      {view === "binder"  && <BinderView pitches={PITCHES} achievements={achievements} onBack={() => setView("map")} />}

      <TweaksPanel title="Tweaks">
        <TweakSection title="Spelkänsla">
          <TweakSlider label="Spelarhastighet" value={tweaks.playerSpeed} min={0.04} max={0.25} step={0.01} onChange={v => setTweaks({ playerSpeed: v })} />
          <TweakRadio label="Kartans tema" value={tweaks.mapTheme} options={[{value:"day",label:"Dag"},{value:"dusk",label:"Skymning"},{value:"night",label:"Natt"}]} onChange={v => setTweaks({ mapTheme: v })} />
          <TweakToggle label="Visa rutnät" value={tweaks.showGrid} onChange={v => setTweaks({ showGrid: v })} />
        </TweakSection>
      </TweaksPanel>
    </div>
  );
}

window.App = App;
