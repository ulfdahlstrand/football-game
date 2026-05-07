const { useState: useStateA, useEffect: useEffectA, useCallback: useCallbackA, useRef: useRefA } = React;

function WorldConfetti({ pitch, player, onDone }) {
  const canvasRef = useRefA(null);
  const TILE = window.TILE || 40;

  useEffectA(() => {
    const canvas = canvasRef.current;
    if (!canvas || !pitch) return;
    const ctx = canvas.getContext('2d');

    const resize = () => {
      canvas.width = window.innerWidth;
      canvas.height = window.innerHeight;
    };
    resize();
    window.addEventListener('resize', resize);

    const COLORS = ['#ffd43b', '#ff6b35', '#4ecdc4', '#45b7d1', '#ff4757', '#2ed573', '#ffffff', '#ff6b9d', '#a29bfe'];

    // Pitch screen position (camera: world container centered at 50%/50%, translated by -camX/-camY)
    const getPitchScreen = () => ({
      x: pitch.gx * TILE - player.px + window.innerWidth / 2,
      y: pitch.gy * TILE - player.py + window.innerHeight / 2
    });

    const particles = [];

    const addConfetti = (count, cx, cy) => {
      for (let i = 0; i < count; i++) {
        particles.push({
          type: 'confetti',
          x: cx + (Math.random() - 0.5) * pitch.w * TILE,
          y: cy + (Math.random() - 0.5) * pitch.h * TILE,
          vx: (Math.random() - 0.5) * 7,
          vy: -4 - Math.random() * 6,
          rot: Math.random() * Math.PI * 2,
          rotV: (Math.random() - 0.5) * 0.3,
          w: 8 + Math.random() * 9,
          h: 4 + Math.random() * 5,
          color: COLORS[Math.floor(Math.random() * COLORS.length)],
          life: 1,
          decay: 0.007 + Math.random() * 0.006
        });
      }
    };

    const launchFirework = (cx, cy) => {
      const color = COLORS[Math.floor(Math.random() * COLORS.length)];
      const count = 40 + Math.floor(Math.random() * 20);
      for (let i = 0; i < count; i++) {
        const angle = (i / count) * Math.PI * 2;
        const speed = 4 + Math.random() * 7;
        particles.push({
          type: 'spark',
          x: cx, y: cy,
          vx: Math.cos(angle) * speed,
          vy: Math.sin(angle) * speed,
          w: 2.5 + Math.random() * 2,
          color,
          life: 1,
          decay: 0.016 + Math.random() * 0.016
        });
      }
    };

    const { x: sx, y: sy } = getPitchScreen();
    addConfetti(120, sx, sy);
    launchFirework(sx, sy - 30);
    launchFirework(sx - 40, sy - 20);
    launchFirework(sx + 40, sy - 20);

    let frame = 0;
    let animId;
    const DURATION = 280; // frames ~4.7s at 60fps

    const animate = () => {
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      frame++;

      const { x: ox, y: oy } = getPitchScreen();

      if (frame % 50 === 0 && frame < DURATION) {
        launchFirework(ox + (Math.random() - 0.5) * pitch.w * TILE * 0.6, oy - 20);
      }
      if (frame % 25 === 0 && frame < DURATION * 0.7) {
        addConfetti(18, ox, oy);
      }

      for (let i = particles.length - 1; i >= 0; i--) {
        const p = particles[i];
        p.x += p.vx;
        p.y += p.vy;
        if (p.type === 'confetti') {
          p.rot += p.rotV;
          p.vy += 0.15;
          p.vx *= 0.98;
        } else {
          p.vy += 0.12;
          p.vx *= 0.97;
        }
        p.life -= p.decay;
        if (p.life <= 0 || p.y > canvas.height + 40) { particles.splice(i, 1); continue; }

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

      if (frame < DURATION || particles.length > 0) {
        animId = requestAnimationFrame(animate);
      } else {
        onDone && onDone();
      }
    };

    animate();
    return () => {
      cancelAnimationFrame(animId);
      window.removeEventListener('resize', resize);
    };
  }, [pitch]);

  return (
    <canvas ref={canvasRef} style={{
      position: 'absolute', inset: 0, pointerEvents: 'none', zIndex: 60
    }} />
  );
}

function App() {
  const TILE = window.TILE || 40;
  const [player, setPlayer] = useStateA({
    px: 14 * TILE,  // pixel-koordinat (mitten av världen)
    py: 10 * TILE,
    dir: 'down',
    moving: false,
    step: false
  });
  const [view, setView] = useStateA('world');
  const [currentMatch, setCurrentMatch] = useStateA(null);
  const [completed, setCompleted] = useStateA([]);
  const [keys, setKeys] = useStateA({});
  const [justWonPitch, setJustWonPitch] = useStateA(null);
  const stepCounterRef = useRefA(0);

  const [tweaks, setTweak] = window.useTweaks(/*EDITMODE-BEGIN*/{
    "playerSpeed": 2.2,
    "showMinimap": true,
    "showHints": true
  }/*EDITMODE-END*/);

  // Tangentbordshantering
  useEffectA(() => {
    const down = (e) => {
      setKeys(k => ({ ...k, [e.key.toLowerCase()]: true }));
      if (e.key.toLowerCase() === 'e' && view === 'world') {
        const playerGx = player.px / TILE;
        const playerGy = player.py / TILE;
        // Skyltar har prioritet (quiz)
        const nearSign = window.SIGN_POSITIONS?.find(s =>
          Math.abs(s.gx - playerGx) <= 1.2 && Math.abs(s.gy - playerGy) <= 1.2
        );
        if (nearSign) { enterSign(nearSign.id); return; }
        // Sedan planer (fotbollsmatch)
        const nearPitch = window.PITCH_POSITIONS.find(p => {
          const dx = Math.abs(p.gx - playerGx);
          const dy = Math.abs(p.gy - playerGy);
          return dx <= (p.w/2 + 0.8) && dy <= (p.h/2 + 0.8);
        });
        if (nearPitch) enterPitch(nearPitch.id);
      }
      if (e.key.toLowerCase() === 'b' && view === 'world') setView('binder');
    };
    const up = (e) => setKeys(k => ({ ...k, [e.key.toLowerCase()]: false }));
    window.addEventListener('keydown', down);
    window.addEventListener('keyup', up);
    return () => {
      window.removeEventListener('keydown', down);
      window.removeEventListener('keyup', up);
    };
  }, [player, view]);

  // Rörelseloop - pixelbaserad, kollisionskoll mot dekorationer och världsgränser
  useEffectA(() => {
    if (view !== 'world') return;
    let raf;
    const tick = () => {
      setPlayer(p => {
        const s = tweaks.playerSpeed;
        let dx = 0, dy = 0, dir = p.dir;
        if (keys['w'] || keys['arrowup']) { dy -= s; dir = 'up'; }
        else if (keys['s'] || keys['arrowdown']) { dy += s; dir = 'down'; }
        else if (keys['a'] || keys['arrowleft']) { dx -= s; dir = 'left'; }
        else if (keys['d'] || keys['arrowright']) { dx += s; dir = 'right'; }

        const moving = dx !== 0 || dy !== 0;
        let npx = p.px + dx;
        let npy = p.py + dy;

        // Världsgräns
        npx = Math.max(TILE * 0.5, Math.min((window.WORLD_W - 0.5) * TILE, npx));
        npy = Math.max(TILE * 0.5, Math.min((window.WORLD_H - 0.5) * TILE, npy));

        // Kollision mot dekorationer (träd, sten, skylt)
        const gx = Math.floor(npx / TILE);
        const gy = Math.floor(npy / TILE);
        const blocked = window.DECORATIONS.some(d =>
          (d.type === 'tree' || d.type === 'rock' || d.type === 'sign') &&
          d.gx === gx && d.gy === gy
        );
        if (blocked) { npx = p.px; npy = p.py; }

        // Kollision mot vatten
        const world = window.__worldMap || (window.__worldMap = window.buildWorld());
        if (world[gy] && world[gy][gx] === 'water') {
          npx = p.px; npy = p.py;
        }

        // Step-animation
        let step = p.step;
        if (moving) {
          stepCounterRef.current += 1;
          if (stepCounterRef.current % 10 === 0) step = !step;
        } else {
          stepCounterRef.current = 0;
        }

        return { px: npx, py: npy, dir, moving, step };
      });
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [keys, view, tweaks.playerSpeed]);

  const enterPitch = (id) => {
    const m = window.MATCH_DATA.find(x => x.id === id);
    setCurrentMatch(m);
    setView('match');
  };

  const enterSign = (id) => {
    const m = window.MATCH_DATA.find(x => x.id === id);
    setCurrentMatch(m);
    setView('quiz');
  };

  const handleMatchComplete = (won, id) => {
    if (won && !completed.includes(id)) {
      setCompleted(c => [...c, id]);
    }
    if (won) {
      const pitchData = window.PITCH_POSITIONS.find(p => p.id === id);
      setJustWonPitch(pitchData || null);
    }
    setView('world');
    setCurrentMatch(null);
  };

  // Pixelerad-bakgrund för hela scenen
  return (
    <div style={{
      position: 'absolute', inset: 0, background: '#1a1a1a', overflow: 'hidden',
      fontFamily: '-apple-system, "Helvetica Neue", Helvetica, Arial, sans-serif',
      imageRendering: 'pixelated'
    }}>
      {view === 'world' && (
        <>
          <window.IsoWorld player={player} onEnterPitch={enterPitch} onEnterSign={enterSign} completedMatches={completed} />
          {justWonPitch && (
            <WorldConfetti
              pitch={justWonPitch}
              player={player}
              onDone={() => setJustWonPitch(null)}
            />
          )}

          {/* HUD - Pokémon-stil överst till vänster */}
          <div style={{
            position: 'absolute', top: 16, left: 16, zIndex: 50,
            background: '#fdfcf7',
            border: '3px solid #1a1a1a',
            borderRadius: 8,
            boxShadow: 'inset 0 0 0 2px #fdfcf7, inset 0 0 0 4px #3464a8, 4px 4px 0 rgba(0,0,0,0.25)',
            padding: '10px 16px'
          }}>
            <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 9, letterSpacing: '0.2em', color: '#3464a8', fontWeight: 700 }}>
              ▸ SÄSONG 01
            </div>
            <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 18, fontWeight: 700, color: '#1a1a1a', lineHeight: 1.1, marginTop: 2 }}>
              {completed.length}/{window.MATCH_DATA.length} ★
            </div>
          </div>

          <button onClick={() => setView('binder')} style={{
            position: 'absolute', top: 16, right: 16, zIndex: 50,
            background: '#e84820',
            color: '#fff',
            border: '3px solid #1a1a1a',
            borderRadius: 8,
            boxShadow: 'inset 0 0 0 2px #e84820, inset 0 0 0 4px #fff, 0 4px 0 #1a1a1a',
            padding: '10px 18px',
            fontFamily: 'ui-monospace, Menlo, monospace',
            fontWeight: 700, fontSize: 11, letterSpacing: '0.15em', cursor: 'pointer',
            display: 'flex', alignItems: 'center', gap: 8
          }}>
            ▸ PÄRMEN [B]
          </button>

          {/* Kontrollhjälp */}
          {tweaks.showHints && (
            <div style={{
              position: 'absolute', top: 16, left: '50%', transform: 'translateX(-50%)', zIndex: 50,
              background: 'rgba(26,26,26,0.88)', color: '#fff',
              padding: '8px 14px',
              borderRadius: 6,
              fontFamily: 'ui-monospace, Menlo, monospace',
              fontSize: 10, letterSpacing: '0.1em', display: 'flex', gap: 14
            }}>
              <span><kbd style={kbdStyle}>WASD</kbd>RÖRELSE</span>
              <span><kbd style={kbdStyle}>E</kbd>INTERAGERA</span>
              <span><kbd style={kbdStyle}>B</kbd>PÄRM</span>
            </div>
          )}

          {/* Minimap */}
          {tweaks.showMinimap && (
            <div style={{
              position: 'absolute', bottom: 16, right: 16, zIndex: 50,
              width: 160, background: '#fdfcf7',
              border: '3px solid #1a1a1a',
              borderRadius: 6,
              boxShadow: 'inset 0 0 0 2px #fdfcf7, inset 0 0 0 4px #3464a8, 4px 4px 0 rgba(0,0,0,0.25)',
              padding: 8
            }}>
              <div style={{
                fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 9,
                letterSpacing: '0.15em', color: '#3464a8', fontWeight: 700, marginBottom: 4
              }}>▸ KARTA</div>
              <div style={{ position: 'relative', width: '100%', aspectRatio: `${window.WORLD_W}/${window.WORLD_H}`, background: '#7ec850', border: '2px solid #1a1a1a' }}>
                {/* Väg */}
                <div style={{ position: 'absolute', left: 0, right: 0, top: '47%', height: '6%', background: '#e8d088' }} />
                {/* Vatten */}
                <div style={{ position: 'absolute', left: '50%', top: '42%', width: '10%', height: '16%', background: '#4aa8d8', transform: 'translateX(-50%)' }} />
                {window.PITCH_POSITIONS.map(p => {
                  const m = window.MATCH_DATA.find(x => x.id === p.id);
                  return (
                    <div key={p.id} style={{
                      position: 'absolute',
                      left: `${((p.gx - p.w/2)/window.WORLD_W)*100}%`,
                      top: `${((p.gy - p.h/2)/window.WORLD_H)*100}%`,
                      width: `${(p.w/window.WORLD_W)*100}%`,
                      height: `${(p.h/window.WORLD_H)*100}%`,
                      background: completed.includes(p.id) ? '#ffd428' : m.color,
                      border: '1px solid #1a1a1a'
                    }} />
                  );
                })}
                <div style={{
                  position: 'absolute',
                  left: `${(player.px/(window.WORLD_W*TILE))*100}%`,
                  top: `${(player.py/(window.WORLD_H*TILE))*100}%`,
                  width: 8, height: 8, background: '#e84820',
                  border: '1.5px solid #1a1a1a', borderRadius: '50%',
                  transform: 'translate(-50%,-50%)',
                  boxShadow: '0 0 0 1px #fff'
                }} />
              </div>
            </div>
          )}
        </>
      )}

      {view === 'match' && currentMatch && (
        <window.FootballMatch
          matchData={currentMatch}
          onComplete={handleMatchComplete}
          onExit={() => { setView('world'); setCurrentMatch(null); }}
        />
      )}

      {view === 'quiz' && currentMatch && (
        <window.MatchScreen
          matchData={currentMatch}
          onComplete={handleMatchComplete}
          onExit={() => { setView('world'); setCurrentMatch(null); }}
        />
      )}

      {view === 'binder' && (
        <window.Binder completedMatches={completed} onClose={() => setView('world')} />
      )}

      {window.TweaksPanel && (
        <window.TweaksPanel title="Tweaks">
          <window.TweakSection title="Spel">
            <window.TweakSlider
              label="Spelarhastighet" value={tweaks.playerSpeed}
              min={0.5} max={5} step={0.1}
              onChange={(v) => setTweak('playerSpeed', v)}
            />
            <window.TweakToggle
              label="Visa minimap" value={tweaks.showMinimap}
              onChange={(v) => setTweak('showMinimap', v)}
            />
            <window.TweakToggle
              label="Visa kontrollhjälp" value={tweaks.showHints}
              onChange={(v) => setTweak('showHints', v)}
            />
          </window.TweakSection>
        </window.TweaksPanel>
      )}
    </div>
  );
}

const kbdStyle = {
  display: 'inline-block', background: '#fdfcf7', color: '#1a1a1a',
  padding: '2px 6px', marginRight: 6, fontSize: 9, fontWeight: 700,
  borderRadius: 3, border: '1px solid #fdfcf7'
};

ReactDOM.createRoot(document.getElementById('root')).render(<App />);
