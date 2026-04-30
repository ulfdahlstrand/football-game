// Samlarkort-pärm
const { useState: useStateB } = React;

function CollectorCard({ matchData, unlocked, onOpen }) {
  if (!unlocked) {
    return (
      <div style={{
        width: 180, height: 250,
        background: 'repeating-linear-gradient(135deg, #e8e4d8 0 8px, #ddd8c8 8px 16px)',
        border: '3px dashed #aaa',
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 10,
        letterSpacing: '0.15em', color: '#888'
      }}>
        LÅST
      </div>
    );
  }
  return (
    <button onClick={onOpen} style={{
      width: 180, height: 250, padding: 0, border: 'none', cursor: 'pointer',
      background: 'transparent', position: 'relative'
    }}>
      <div style={{
        position: 'absolute', inset: 0,
        background: `linear-gradient(180deg, ${matchData.color} 0%, #2d5a23 100%)`,
        border: '3px solid #1a1a1a',
        boxShadow: '5px 5px 0 #1a1a1a',
        display: 'flex', flexDirection: 'column',
        transition: 'transform 0.15s'
      }}
      onMouseEnter={(e) => e.currentTarget.style.transform = 'translate(-3px,-3px)'}
      onMouseLeave={(e) => e.currentTarget.style.transform = 'none'}
      >
        {/* Övre band */}
        <div style={{
          background: '#1a1a1a', color: '#fff', padding: '6px 10px',
          fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 9,
          letterSpacing: '0.15em', display: 'flex', justifyContent: 'space-between'
        }}>
          <span>#{String(matchData.id).toUpperCase().slice(0,3)}</span>
          <span>★ {matchData.level}</span>
        </div>
        {/* Illustration */}
        <div style={{
          flex: 1, margin: 10, background: 'rgba(255,255,255,0.12)',
          border: '2px solid rgba(255,255,255,0.4)',
          display: 'flex', alignItems: 'center', justifyContent: 'center',
          position: 'relative'
        }}>
          <svg viewBox="0 0 100 80" style={{ width: '80%' }}>
            <polygon points="50,10 92,40 50,70 8,40" fill="none" stroke="rgba(255,255,255,0.8)" strokeWidth="1.5" />
            <line x1="50" y1="10" x2="50" y2="70" stroke="rgba(255,255,255,0.8)" strokeWidth="1.5" />
            <ellipse cx="50" cy="40" rx="8" ry="5" fill="none" stroke="rgba(255,255,255,0.8)" strokeWidth="1.5" />
            <circle cx="50" cy="40" r="4" fill="#fff" />
          </svg>
        </div>
        {/* Titel */}
        <div style={{
          background: '#fdfcf7', padding: '10px 12px',
          borderTop: '2px solid #1a1a1a'
        }}>
          <div style={{ fontFamily: 'Georgia, serif', fontSize: 14, fontWeight: 700, color: '#1a1a1a', lineHeight: 1.1 }}>
            {matchData.name}
          </div>
          <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 9, letterSpacing: '0.1em', color: '#888', marginTop: 3 }}>
            {matchData.questions.length} LÄRDOMAR
          </div>
        </div>
      </div>
    </button>
  );
}

function CardDetail({ matchData, onClose }) {
  return (
    <div style={{
      position: 'fixed', inset: 0, background: 'rgba(0,0,0,0.7)', zIndex: 200,
      display: 'flex', alignItems: 'center', justifyContent: 'center', padding: 24
    }} onClick={onClose}>
      <div onClick={e => e.stopPropagation()} style={{
        background: '#fdfcf7', border: '3px solid #1a1a1a',
        boxShadow: '10px 10px 0 rgba(0,0,0,0.4)',
        maxWidth: 600, width: '100%', maxHeight: '85vh', overflow: 'auto'
      }}>
        <div style={{
          background: matchData.color, padding: '20px 24px', borderBottom: '3px solid #1a1a1a',
          display: 'flex', justifyContent: 'space-between', alignItems: 'center'
        }}>
          <div>
            <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 10, letterSpacing: '0.15em', color: 'rgba(255,255,255,0.85)' }}>
              SAMLARKORT · {matchData.level.toUpperCase()}
            </div>
            <div style={{ fontFamily: 'Georgia, serif', fontSize: 26, fontWeight: 700, color: '#fff' }}>
              {matchData.name}
            </div>
          </div>
          <button onClick={onClose} style={{
            background: '#1a1a1a', color: '#fff', border: 'none', width: 36, height: 36,
            fontSize: 18, cursor: 'pointer'
          }}>×</button>
        </div>
        <div style={{ padding: 24 }}>
          <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 10, letterSpacing: '0.15em', color: '#666', marginBottom: 14 }}>
            LÄRDOMAR FRÅN MATCHEN
          </div>
          {matchData.questions.map((q, i) => (
            <div key={i} style={{ marginBottom: 22, paddingBottom: 22, borderBottom: i < matchData.questions.length - 1 ? '1px dashed #ccc' : 'none' }}>
              <div style={{
                fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 10, letterSpacing: '0.15em',
                color: matchData.color, marginBottom: 6, fontWeight: 700
              }}>
                SITUATION {i + 1}
              </div>
              <div style={{ fontFamily: 'Georgia, serif', fontSize: 16, color: '#1a1a1a', marginBottom: 10, lineHeight: 1.4, textWrap: 'pretty' }}>
                {q.q}
              </div>
              <div style={{
                background: '#c7e8b8', border: '2px solid #3d7a2e', padding: '8px 12px',
                fontSize: 14, color: '#1a1a1a', marginBottom: 10
              }}>
                <strong>✓ </strong>{q.options[q.correct]}
              </div>
              <div style={{ fontSize: 13, color: '#555', lineHeight: 1.5, textWrap: 'pretty' }}>
                {q.why}
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

function Binder({ completedMatches, onClose }) {
  const [selected, setSelected] = useStateB(null);
  return (
    <div style={{
      position: 'fixed', inset: 0, background: '#2a2320', zIndex: 150,
      display: 'flex', flexDirection: 'column'
    }}>
      {/* Header som pärmrygg */}
      <div style={{
        background: '#5a3a2a', borderBottom: '4px solid #1a1a1a',
        padding: '18px 28px', display: 'flex', justifyContent: 'space-between', alignItems: 'center',
        backgroundImage: 'repeating-linear-gradient(90deg, rgba(0,0,0,0.08) 0 2px, transparent 2px 8px)'
      }}>
        <div>
          <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 10, letterSpacing: '0.2em', color: 'rgba(255,255,255,0.7)' }}>
            MIN SAMLING · SÄSONG 01
          </div>
          <div style={{ fontFamily: 'Georgia, serif', fontSize: 28, fontWeight: 700, color: '#fdfcf7' }}>
            Pärmen
          </div>
        </div>
        <div style={{ display: 'flex', gap: 16, alignItems: 'center' }}>
          <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 12, color: '#fff', letterSpacing: '0.1em' }}>
            {completedMatches.length} / {window.MATCH_DATA.length} KORT
          </div>
          <button onClick={onClose} style={{
            background: '#ff6b35', color: '#fff', border: '2px solid #1a1a1a',
            padding: '8px 16px', fontFamily: 'ui-monospace, Menlo, monospace',
            fontWeight: 700, fontSize: 11, letterSpacing: '0.1em', cursor: 'pointer',
            boxShadow: '3px 3px 0 #1a1a1a'
          }}>STÄNG ✕</button>
        </div>
      </div>

      {/* Sida */}
      <div style={{
        flex: 1, padding: 32, overflow: 'auto',
        background: '#3d2e25',
        backgroundImage: 'radial-gradient(circle at 20% 20%, rgba(255,255,255,0.04), transparent 60%)'
      }}>
        <div style={{
          maxWidth: 900, margin: '0 auto',
          background: '#fdfcf7',
          border: '3px solid #1a1a1a',
          padding: 40,
          boxShadow: '10px 10px 0 rgba(0,0,0,0.4)',
          position: 'relative'
        }}>
          {/* Hål i pärmen */}
          {[0,1,2,3].map(i => (
            <div key={i} style={{
              position: 'absolute', left: -3, top: 50 + i * 90,
              width: 14, height: 14, background: '#2a2320',
              border: '2px solid #1a1a1a', borderRadius: '50%'
            }} />
          ))}
          <div style={{
            display: 'grid', gridTemplateColumns: 'repeat(auto-fill, 180px)',
            gap: 28, justifyContent: 'center'
          }}>
            {window.MATCH_DATA.map(m => (
              <CollectorCard
                key={m.id}
                matchData={m}
                unlocked={completedMatches.includes(m.id)}
                onOpen={() => setSelected(m)}
              />
            ))}
          </div>
          {completedMatches.length === 0 && (
            <div style={{
              marginTop: 30, padding: 20, background: '#f5f3ed',
              border: '2px dashed #aaa', textAlign: 'center',
              fontFamily: 'Georgia, serif', fontStyle: 'italic', color: '#666'
            }}>
              Dina samlarkort dyker upp här när du vunnit matcher.
            </div>
          )}
        </div>
      </div>

      {selected && <CardDetail matchData={selected} onClose={() => setSelected(null)} />}
    </div>
  );
}

window.Binder = Binder;
