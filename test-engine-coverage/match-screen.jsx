// Matchvy - frågor + straffar
const { useState: useStateM, useEffect: useEffectM, useRef: useRefM } = React;

const ROLE_BADGE = { fwd: 'ANF', mid: 'MIT', def: 'BAC', gk: 'MÅL' };

function LineupScreen({ matchData, onKickoff }) {
  const [roster, setRoster] = useStateM(null);

  useEffectM(() => {
    if (!matchData.team) { onKickoff(); return; }
    fetch(`data/teams/${matchData.team}/roster.json?t=${Date.now()}`, { cache: 'no-store' })
      .then(r => r.json())
      .then(setRoster)
      .catch(onKickoff);
  }, []);

  if (!roster) {
    return (
      <div style={{
        position: 'absolute', inset: 0,
        background: `linear-gradient(180deg, ${matchData.color} 0%, #1a1a1a 100%)`,
        display: 'flex', alignItems: 'center', justifyContent: 'center'
      }}>
        <div style={{ color: 'rgba(255,255,255,0.5)', fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 11, letterSpacing: '0.2em' }}>
          LADDAR LAG…
        </div>
      </div>
    );
  }

  return (
    <div style={{
      position: 'absolute', inset: 0,
      background: `linear-gradient(180deg, ${matchData.color} 0%, #1a1a1a 100%)`,
      display: 'flex', flexDirection: 'column', overflow: 'auto'
    }}>
      {/* Planlinjer-bakgrund */}
      <svg style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', opacity: 0.08, pointerEvents: 'none' }}>
        <line x1="50%" y1="0" x2="50%" y2="100%" stroke="#fff" strokeWidth="2" />
        <circle cx="50%" cy="50%" r="80" fill="none" stroke="#fff" strokeWidth="2" />
      </svg>

      <div style={{ position: 'relative', maxWidth: 580, width: '100%', margin: '0 auto', padding: '32px 24px 24px' }}>
        {/* Match-header */}
        <div style={{ marginBottom: 24 }}>
          <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 10, letterSpacing: '0.2em', color: 'rgba(255,255,255,0.65)', marginBottom: 12 }}>
            {matchData.level.toUpperCase()} · {matchData.name.toUpperCase()}
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
            <img
              src={`data/teams/${matchData.team}/logo.svg`}
              style={{ width: 64, height: 64, objectFit: 'contain', flexShrink: 0 }}
              alt=""
            />
            <div>
              <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 10, letterSpacing: '0.2em', color: 'rgba(255,255,255,0.65)' }}>
                MOTSTÅNDARE
              </div>
              <div style={{ fontFamily: 'Georgia, serif', fontSize: 32, fontWeight: 700, color: '#fff', lineHeight: 1.1, marginTop: 2 }}>
                {matchData.team.split('-').map(w => w.charAt(0).toUpperCase() + w.slice(1)).join(' ')}
              </div>
            </div>
          </div>
        </div>

        {/* Lagbeskrivning */}
        {roster.description && (
          <div style={{
            background: 'rgba(0,0,0,0.35)', border: '1px solid rgba(255,255,255,0.15)',
            padding: '14px 16px', marginBottom: 20
          }}>
            <div style={{ fontFamily: 'Georgia, serif', fontSize: 14, color: 'rgba(255,255,255,0.9)', lineHeight: 1.6, fontStyle: 'italic' }}>
              {roster.description}
            </div>
          </div>
        )}

        {/* Spelarlista */}
        <div style={{ marginBottom: 28 }}>
          <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 10, letterSpacing: '0.2em', color: 'rgba(255,255,255,0.5)', marginBottom: 12 }}>
            UPPSTÄLLNING
          </div>
          {roster.players.map(p => (
            <div key={p.id} style={{
              display: 'flex', gap: 12, marginBottom: 12,
              background: 'rgba(0,0,0,0.25)', padding: '10px 12px',
              border: '1px solid rgba(255,255,255,0.1)'
            }}>
              <div style={{
                flexShrink: 0, width: 36, height: 36,
                background: 'rgba(255,255,255,0.15)', border: '1px solid rgba(255,255,255,0.3)',
                display: 'flex', alignItems: 'center', justifyContent: 'center',
                fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 8,
                fontWeight: 700, color: '#fff', letterSpacing: '0.05em'
              }}>
                {ROLE_BADGE[p.role]}
              </div>
              <div style={{ minWidth: 0 }}>
                <div style={{ fontFamily: 'Georgia, serif', fontSize: 14, fontWeight: 700, color: '#fff', marginBottom: 2 }}>
                  {p.name}
                </div>
                <div style={{ fontFamily: 'Georgia, serif', fontSize: 12, color: 'rgba(255,255,255,0.65)', lineHeight: 1.45, fontStyle: 'italic' }}>
                  {p.description}
                </div>
              </div>
            </div>
          ))}
        </div>

        {/* Kickoff-knapp */}
        <button onClick={onKickoff} style={{
          width: '100%', background: '#fff', color: '#1a1a1a',
          border: '3px solid #1a1a1a', padding: '16px 24px',
          fontFamily: 'ui-monospace, Menlo, monospace', fontWeight: 700,
          fontSize: 14, letterSpacing: '0.2em', cursor: 'pointer',
          boxShadow: '5px 5px 0 rgba(0,0,0,0.4)'
        }}>
          SPARKA AV ▸
        </button>
      </div>
    </div>
  );
}

function QuestionCard({ question, qIndex, total, onAnswer, locked, selected, correct }) {
  return (
    <div style={{
      background: '#fdfcf7',
      border: '3px solid #1a1a1a',
      padding: 28,
      maxWidth: 560,
      width: '100%',
      boxShadow: '8px 8px 0 #1a1a1a'
    }}>
      <div style={{
        display: 'flex',
        justifyContent: 'space-between',
        marginBottom: 18,
        fontFamily: 'ui-monospace, Menlo, monospace',
        fontSize: 11,
        letterSpacing: '0.12em',
        textTransform: 'uppercase',
        color: '#666'
      }}>
        <span>Fråga {qIndex + 1} / {total}</span>
        <span>▌▌▌▌ Situation</span>
      </div>
      <h2 style={{
        fontFamily: 'Georgia, serif',
        fontSize: 22,
        lineHeight: 1.35,
        margin: '0 0 24px',
        color: '#1a1a1a',
        textWrap: 'pretty'
      }}>
        {question.q}
      </h2>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
        {question.options.map((opt, i) => {
          const isSelected = selected === i;
          const isCorrect = correct === i;
          let bg = '#fff', borderCol = '#1a1a1a', color = '#1a1a1a';
          if (locked) {
            if (isCorrect) { bg = '#c7e8b8'; borderCol = '#3d7a2e'; }
            else if (isSelected && !isCorrect) { bg = '#f5c8c0'; borderCol = '#b83a2a'; }
            else { bg = '#f5f3ed'; color: '#888'; }
          }
          return (
            <button
              key={i}
              onClick={() => !locked && onAnswer(i)}
              disabled={locked}
              style={{
                background: bg,
                border: `2px solid ${borderCol}`,
                padding: '14px 16px',
                textAlign: 'left',
                fontFamily: 'inherit',
                fontSize: 15,
                color,
                cursor: locked ? 'default' : 'pointer',
                display: 'flex',
                gap: 12,
                alignItems: 'center',
                transition: 'transform 0.1s'
              }}
              onMouseEnter={(e) => !locked && (e.currentTarget.style.transform = 'translate(-2px,-2px)')}
              onMouseLeave={(e) => !locked && (e.currentTarget.style.transform = 'none')}
            >
              <span style={{
                width: 26, height: 26, display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
                background: '#1a1a1a', color: '#fff', fontFamily: 'ui-monospace, Menlo, monospace',
                fontSize: 12, fontWeight: 700, flexShrink: 0
              }}>{String.fromCharCode(65+i)}</span>
              <span style={{ textWrap: 'pretty' }}>{opt}</span>
            </button>
          );
        })}
      </div>
    </div>
  );
}

function MatchScreen({ matchData, onComplete, onExit }) {
  const [qIndex, setQIndex] = useStateM(0);
  const [selected, setSelected] = useStateM(null);
  const [locked, setLocked] = useStateM(false);
  const [score, setScore] = useStateM(0);
  const [phase, setPhase] = useStateM('lineup'); // lineup | question | feedback | penalty | done
  const [result, setResult] = useStateM(null); // 'win' | 'loss' | 'penalty'

  const q = matchData.questions[qIndex];

  const handleAnswer = (i) => {
    setSelected(i);
    setLocked(true);
    if (i === q.correct) setScore(s => s + 1);
    setTimeout(() => setPhase('feedback'), 400);
  };

  const next = () => {
    if (qIndex + 1 < matchData.questions.length) {
      setQIndex(qIndex + 1);
      setSelected(null);
      setLocked(false);
      setPhase('question');
    } else {
      // färdigt - avgör resultat
      const finalScore = score;
      if (finalScore >= 3) {
        setResult('win');
        setPhase('done');
      } else if (finalScore === 2) {
        setResult('penalty');
        setPhase('penalty');
      } else {
        setResult('loss');
        setPhase('done');
      }
    }
  };

  if (phase === 'lineup') {
    return <LineupScreen matchData={matchData} onKickoff={() => setPhase('question')} />;
  }

  if (phase === 'penalty') {
    return <PenaltyShootout matchData={matchData} onFinish={(won) => {
      setResult(won ? 'win' : 'loss');
      setPhase('done');
    }} />;
  }

  if (phase === 'done') {
    return <ResultScreen
      result={result}
      matchData={matchData}
      score={score}
      onExit={onExit}
      onComplete={() => onComplete(result === 'win', matchData.id)}
    />;
  }

  return (
    <div style={{
      position: 'absolute', inset: 0,
      background: `linear-gradient(180deg, ${matchData.color} 0%, #2d5a23 100%)`,
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      padding: 24, overflow: 'auto'
    }}>
      {/* Planlinjer-bakgrund */}
      <svg style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', opacity: 0.15 }}>
        <line x1="50%" y1="0" x2="50%" y2="100%" stroke="#fff" strokeWidth="2" />
        <circle cx="50%" cy="50%" r="80" fill="none" stroke="#fff" strokeWidth="2" />
      </svg>

      <div style={{ position: 'relative', width: '100%', maxWidth: 620, display: 'flex', flexDirection: 'column', gap: 18 }}>
        {/* Header */}
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-end', color: '#fff' }}>
          <div>
            <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 10, letterSpacing: '0.15em', opacity: 0.8 }}>
              {matchData.level.toUpperCase()}
            </div>
            <div style={{ fontFamily: 'Georgia, serif', fontSize: 26, fontWeight: 700 }}>
              {matchData.name}
            </div>
          </div>
          <div style={{ display: 'flex', gap: 6 }}>
            {matchData.questions.map((_, i) => (
              <div key={i} style={{
                width: 18, height: 18, border: '2px solid #fff',
                background: i < qIndex ? '#fff' : (i === qIndex ? 'rgba(255,255,255,0.3)' : 'transparent')
              }} />
            ))}
          </div>
        </div>

        {phase === 'question' && (
          <QuestionCard
            question={q} qIndex={qIndex} total={matchData.questions.length}
            onAnswer={handleAnswer} locked={locked}
            selected={selected} correct={locked ? q.correct : null}
          />
        )}

        {phase === 'feedback' && (
          <div style={{
            background: '#fdfcf7', border: '3px solid #1a1a1a', padding: 24,
            boxShadow: '8px 8px 0 #1a1a1a'
          }}>
            <div style={{
              fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 11,
              letterSpacing: '0.12em', color: selected === q.correct ? '#3d7a2e' : '#b83a2a',
              marginBottom: 10, fontWeight: 700
            }}>
              {selected === q.correct ? '✓ RÄTT SVAR' : '✕ FEL SVAR'}
            </div>
            <div style={{ fontFamily: 'Georgia, serif', fontSize: 16, lineHeight: 1.5, color: '#1a1a1a', marginBottom: 18, textWrap: 'pretty' }}>
              <strong>Rätt svar: </strong>{q.options[q.correct]}
            </div>
            <div style={{ fontSize: 14, lineHeight: 1.55, color: '#444', marginBottom: 20, textWrap: 'pretty' }}>
              {q.why}
            </div>
            <button onClick={next} style={{
              background: '#1a1a1a', color: '#fff', border: 'none', padding: '12px 20px',
              fontFamily: 'ui-monospace, Menlo, monospace', fontWeight: 700, fontSize: 12,
              letterSpacing: '0.1em', cursor: 'pointer'
            }}>
              {qIndex + 1 < matchData.questions.length ? 'NÄSTA FRÅGA ▸' : 'SE RESULTAT ▸'}
            </button>
          </div>
        )}

        <button onClick={onExit} style={{
          alignSelf: 'flex-start', background: 'transparent', color: '#fff',
          border: '1px solid rgba(255,255,255,0.5)', padding: '6px 12px',
          fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 10,
          letterSpacing: '0.1em', cursor: 'pointer'
        }}>← LÄMNA PLANEN</button>
      </div>
    </div>
  );
}

// STRAFFMINISPEL
function PenaltyShootout({ matchData, onFinish }) {
  const [phase, setPhase] = useStateM('aim'); // aim | shooting | result
  const [choice, setChoice] = useStateM(null);
  const [keeperGo, setKeeperGo] = useStateM(null);
  const [won, setWon] = useStateM(null);

  const shoot = (direction) => {
    setChoice(direction);
    setPhase('shooting');
    // Slumpmässig målvakt
    const dirs = ['left', 'mid', 'right'];
    const gk = dirs[Math.floor(Math.random() * 3)];
    setKeeperGo(gk);
    setTimeout(() => {
      const didWin = gk !== direction;
      setWon(didWin);
      setPhase('result');
    }, 1400);
  };

  return (
    <div style={{
      position: 'absolute', inset: 0,
      background: 'linear-gradient(180deg, #87ceeb 0%, #4a8d3a 55%, #3d7a2e 100%)',
      display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center',
      padding: 24
    }}>
      <div style={{ color: '#fff', fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 11, letterSpacing: '0.15em', marginBottom: 8 }}>
        OAVGJORT 2-2 · STRAFFAVGÖRANDE
      </div>
      <div style={{ color: '#fff', fontFamily: 'Georgia, serif', fontSize: 32, fontWeight: 700, marginBottom: 30 }}>
        Sista sparken
      </div>

      {/* Mål */}
      <div style={{ position: 'relative', width: 420, height: 180 }}>
        {/* Målram */}
        <div style={{
          position: 'absolute', inset: 0, border: '6px solid #fff',
          borderBottom: 'none', background: 'rgba(0,0,0,0.15)',
          backgroundImage: 'repeating-linear-gradient(0deg, rgba(255,255,255,0.15) 0 1px, transparent 1px 14px), repeating-linear-gradient(90deg, rgba(255,255,255,0.15) 0 1px, transparent 1px 14px)'
        }} />
        {/* Målvakt */}
        <div style={{
          position: 'absolute',
          bottom: 12,
          left: phase === 'shooting' || phase === 'result' ?
            (keeperGo === 'left' ? 40 : keeperGo === 'right' ? 320 : 180) : 180,
          width: 60, height: 90,
          transition: 'left 0.5s cubic-bezier(.3,1.4,.5,1)',
          display: 'flex', flexDirection: 'column', alignItems: 'center'
        }}>
          <div style={{ width: 20, height: 20, background: '#f4c896', border: '2px solid #1a1a1a', borderRadius: '50%' }} />
          <div style={{ width: 36, height: 50, background: '#ffd43b', border: '2px solid #1a1a1a', marginTop: -2 }} />
          <div style={{ display: 'flex', gap: 4 }}>
            <div style={{ width: 10, height: 16, background: '#1a1a1a' }} />
            <div style={{ width: 10, height: 16, background: '#1a1a1a' }} />
          </div>
        </div>
        {/* Boll */}
        {phase !== 'aim' && (
          <div style={{
            position: 'absolute',
            bottom: phase === 'result' && won ? 120 : -80,
            left: choice === 'left' ? 50 : choice === 'right' ? 320 : 195,
            width: 24, height: 24,
            background: '#fff',
            border: '2px solid #1a1a1a',
            borderRadius: '50%',
            transition: 'all 1.2s cubic-bezier(.5,.1,.8,1)',
            boxShadow: '2px 2px 0 rgba(0,0,0,0.3)'
          }} />
        )}
      </div>

      {/* Straffpunkt */}
      <div style={{ width: 12, height: 12, background: '#fff', borderRadius: '50%', marginTop: 60 }} />

      {phase === 'aim' && (
        <div style={{ marginTop: 40, display: 'flex', gap: 12 }}>
          {[
            { k: 'left', label: '◄ VÄNSTER' },
            { k: 'mid', label: '▲ MITTEN' },
            { k: 'right', label: 'HÖGER ►' }
          ].map(b => (
            <button key={b.k} onClick={() => shoot(b.k)} style={{
              background: '#ff6b35', color: '#fff', border: '3px solid #1a1a1a',
              padding: '14px 22px', fontFamily: 'ui-monospace, Menlo, monospace',
              fontWeight: 700, fontSize: 12, letterSpacing: '0.1em', cursor: 'pointer',
              boxShadow: '4px 4px 0 #1a1a1a'
            }}>{b.label}</button>
          ))}
        </div>
      )}

      {phase === 'result' && (
        <div style={{ marginTop: 30, textAlign: 'center' }}>
          <div style={{
            color: '#fff', fontFamily: 'Georgia, serif', fontSize: 36, fontWeight: 700,
            marginBottom: 18, textShadow: '3px 3px 0 #1a1a1a'
          }}>
            {won ? 'MÅÅÅL!' : 'RÄDDAD!'}
          </div>
          <button onClick={() => onFinish(won)} style={{
            background: '#1a1a1a', color: '#fff', border: 'none', padding: '12px 22px',
            fontFamily: 'ui-monospace, Menlo, monospace', fontWeight: 700, fontSize: 12,
            letterSpacing: '0.1em', cursor: 'pointer'
          }}>FORTSÄTT ▸</button>
        </div>
      )}
    </div>
  );
}

function ConfettiFireworks() {
  const canvasRef = useRefM(null);

  useEffectM(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');

    const resize = () => {
      canvas.width = window.innerWidth;
      canvas.height = window.innerHeight;
    };
    resize();
    window.addEventListener('resize', resize);

    const COLORS = ['#ffd43b', '#ff6b35', '#4ecdc4', '#45b7d1', '#ff4757', '#2ed573', '#ffffff', '#ff6b9d', '#a29bfe'];

    const particles = [];

    const addConfettiBurst = (count) => {
      for (let i = 0; i < count; i++) {
        particles.push({
          type: 'confetti',
          x: Math.random() * canvas.width,
          y: -10 - Math.random() * 100,
          vx: (Math.random() - 0.5) * 5,
          vy: 2 + Math.random() * 5,
          rot: Math.random() * Math.PI * 2,
          rotV: (Math.random() - 0.5) * 0.25,
          w: 7 + Math.random() * 9,
          h: 4 + Math.random() * 5,
          color: COLORS[Math.floor(Math.random() * COLORS.length)],
          life: 1,
          decay: 0.003 + Math.random() * 0.004
        });
      }
    };

    const launchFirework = () => {
      const x = canvas.width * 0.15 + Math.random() * canvas.width * 0.7;
      const y = canvas.height * 0.15 + Math.random() * canvas.height * 0.45;
      const color = COLORS[Math.floor(Math.random() * COLORS.length)];
      const count = 36 + Math.floor(Math.random() * 20);
      for (let i = 0; i < count; i++) {
        const angle = (i / count) * Math.PI * 2;
        const speed = 3 + Math.random() * 6;
        particles.push({
          type: 'spark',
          x, y,
          vx: Math.cos(angle) * speed,
          vy: Math.sin(angle) * speed,
          w: 2.5 + Math.random() * 2,
          color,
          life: 1,
          decay: 0.018 + Math.random() * 0.018
        });
      }
    };

    addConfettiBurst(140);
    launchFirework();
    launchFirework();

    let frame = 0;
    let animId;

    const animate = () => {
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      frame++;

      if (frame % 45 === 0) launchFirework();
      if (frame % 28 === 0 && frame < 400) addConfettiBurst(22);

      for (let i = particles.length - 1; i >= 0; i--) {
        const p = particles[i];
        p.x += p.vx;
        p.y += p.vy;
        if (p.type === 'confetti') {
          p.rot += p.rotV;
          p.vy += 0.06;
          p.vx *= 0.99;
        } else {
          p.vy += 0.12;
          p.vx *= 0.97;
        }
        p.life -= p.decay;
        if (p.life <= 0 || p.y > canvas.height + 30) { particles.splice(i, 1); continue; }

        ctx.save();
        ctx.globalAlpha = Math.max(0, p.life);
        ctx.fillStyle = p.color;
        if (p.type === 'confetti') {
          ctx.translate(p.x, p.y);
          ctx.rotate(p.rot);
          ctx.fillRect(-p.w / 2, -p.h / 2, p.w, p.h);
        } else {
          ctx.beginPath();
          ctx.arc(p.x, p.y, p.w, 0, Math.PI * 2);
          ctx.fill();
        }
        ctx.restore();
      }

      animId = requestAnimationFrame(animate);
    };

    animate();
    return () => {
      cancelAnimationFrame(animId);
      window.removeEventListener('resize', resize);
    };
  }, []);

  return (
    <canvas ref={canvasRef} style={{
      position: 'absolute', inset: 0, pointerEvents: 'none', zIndex: 10
    }} />
  );
}

function ResultScreen({ result, matchData, score, onExit, onComplete }) {
  useEffectM(() => {
    if (result === 'win') {
      const t = setTimeout(() => onComplete(), 2400);
      return () => clearTimeout(t);
    }
  }, []);

  return (
    <div style={{
      position: 'absolute', inset: 0,
      background: result === 'win' ? 'linear-gradient(180deg, #ffd43b 0%, #ff6b35 100%)' : '#2a2a2a',
      display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center',
      padding: 24
    }}>
      <div style={{
        fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 11, letterSpacing: '0.2em',
        color: result === 'win' ? '#1a1a1a' : '#888', marginBottom: 12
      }}>
        {matchData.name.toUpperCase()} · {score}/{matchData.questions.length} RÄTT
      </div>
      <div style={{
        fontFamily: 'Georgia, serif', fontSize: 72, fontWeight: 700, color: '#fff',
        textShadow: result === 'win' ? '4px 4px 0 #1a1a1a' : 'none', marginBottom: 20
      }}>
        {result === 'win' ? 'VINST' : 'FÖRLUST'}
      </div>
      <div style={{
        fontFamily: 'Georgia, serif', fontSize: 18, color: result === 'win' ? '#1a1a1a' : '#ccc',
        marginBottom: 30, maxWidth: 420, textAlign: 'center', lineHeight: 1.5
      }}>
        {result === 'win' ? 'Du har låst upp ett nytt samlarkort till pärmen!' : 'Kom tillbaka och försök igen. Övning ger färdighet.'}
      </div>
      {result === 'win' ? (
        <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 11, color: '#1a1a1a', letterSpacing: '0.15em' }}>
          ÖPPNAR PÄRMEN...
        </div>
      ) : (
        <button onClick={onExit} style={{
          background: '#fff', color: '#1a1a1a', border: 'none', padding: '14px 24px',
          fontFamily: 'ui-monospace, Menlo, monospace', fontWeight: 700, fontSize: 12,
          letterSpacing: '0.1em', cursor: 'pointer'
        }}>TILLBAKA TILL KARTAN</button>
      )}
    </div>
  );
}

window.MatchScreen = MatchScreen;
