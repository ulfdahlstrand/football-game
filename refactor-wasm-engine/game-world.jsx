// Top-down Pokémon-stil värld - kaklad, inga isometriska projektioner
const { useState, useEffect, useRef, useCallback } = React;

const TILE = 40; // px per kakel
const WORLD_W = 28;
const WORLD_H = 20;

// Fotbollsplans-positioner (grid-koordinater, mitten av planen)
const PITCH_POSITIONS = [
  { id: 'parken', gx: 5,  gy: 4,  w: 5, h: 3 },
  { id: 'kullen', gx: 22, gy: 4,  w: 5, h: 3 },
  { id: 'arena',  gx: 5,  gy: 15, w: 5, h: 3 },
  { id: 'stadion',gx: 22, gy: 15, w: 5, h: 3 }
];

// Palett (GBA-inspired, saturated)
const PAL = {
  grassA: '#7ec850',
  grassB: '#6db840',
  grassDark: '#5aa82f',
  path: '#e8d088',
  pathDark: '#d4b868',
  water: '#4aa8d8',
  waterDark: '#2e88b8',
  tree: '#2d7a2a',
  treeLight: '#4aa048',
  trunk: '#5a3420',
  flower: '#ff5580',
  flowerY: '#ffd028'
};

// Statisk världskarta - 2d-array med tile-typer
function buildWorld() {
  const map = [];
  for (let y = 0; y < WORLD_H; y++) {
    const row = [];
    for (let x = 0; x < WORLD_W; x++) {
      // Grundgräs - variation
      let t = ((x * 7 + y * 13) % 5 === 0) ? 'grassDark' : ((x + y) % 2 === 0 ? 'grassA' : 'grassB');
      // Horisontell huvudväg
      if (y >= 9 && y <= 10) t = 'path';
      // Vertikala vägar
      if ((x === 5 || x === 22) && y > 5 && y < 15) t = 'path';
      if ((x === 6 || x === 23) && y > 5 && y < 15 && (y % 3 === 0)) t = 'path';
      row.push(t);
    }
    map.push(row);
  }
  // Vatten-damm (liten sjö)
  for (let y = 8; y <= 11; y++) {
    for (let x = 14; x <= 16; x++) {
      if (!((x === 14 && y === 8) || (x === 16 && y === 11))) {
        map[y][x] = 'water';
      }
    }
  }
  return map;
}

// Dekorationsobjekt (träd, blommor, stenar)
const DECORATIONS = [
  // Träd - kantskog
  ...[[0,0],[0,2],[0,4],[0,6],[0,8],[0,12],[0,14],[0,16],[0,18],
      [27,0],[27,3],[27,6],[27,11],[27,14],[27,17],[27,19],
      [2,0],[4,0],[8,0],[12,0],[16,0],[20,0],[24,0],
      [2,19],[6,19],[10,19],[14,19],[18,19],[22,19],[26,19]
  ].map(([x,y]) => ({ type:'tree', gx:x, gy:y })),
  // Träd-klustrar
  ...[[11,6],[12,6],[13,6],[11,13],[16,13],[17,13]].map(([x,y]) => ({ type:'tree', gx:x, gy:y })),
  // Blommor
  ...[[3,5],[4,6],[8,8],[18,7],[19,8],[24,6],[25,5],[3,14],[18,14],[25,13]].map(([x,y],i) => ({
    type:'flower', gx:x, gy:y, color: i % 2 === 0 ? 'pink' : 'yellow'
  })),
  // Stenar
  ...[[13,4],[20,12],[9,16]].map(([x,y]) => ({ type:'rock', gx:x, gy:y })),
  // Skyltar vid planer (med id för quiz-koppling)
  { type:'sign', gx:7,  gy:6,  label:'PARKENS\nGRÄSMATTA', id:'parken'  },
  { type:'sign', gx:21, gy:6,  label:'KULLENS\nIP',          id:'kullen'  },
  { type:'sign', gx:7,  gy:14, label:'NORRA\nARENAN',        id:'arena'   },
  { type:'sign', gx:21, gy:14, label:'STADION',              id:'stadion' }
];

// Kolla om tile är blockerad (vatten, träd, sten)
function isBlocked(map, gx, gy, decorations, pitches) {
  if (gx < 0 || gx >= WORLD_W || gy < 0 || gy >= WORLD_H) return true;
  const t = map[gy]?.[gx];
  if (t === 'water') return true;
  // Kolla dekorationer
  for (const d of decorations) {
    if (d.gx === gx && d.gy === gy && (d.type === 'tree' || d.type === 'rock' || d.type === 'sign')) {
      return true;
    }
  }
  // Planens "insida" är blockerad (spelare går runt den och triggrar via kanten)
  // – vi låter spelare gå på plan-kanten (prompt-zon), inte mitt i
  return false;
}

// Enskild tile-ritning
function Tile({ gx, gy, type }) {
  const colors = {
    grassA: PAL.grassA, grassB: PAL.grassB, grassDark: PAL.grassDark,
    path: PAL.path, water: PAL.water
  };
  const bg = colors[type] || PAL.grassA;
  return (
    <div style={{
      position: 'absolute',
      left: gx * TILE, top: gy * TILE,
      width: TILE, height: TILE,
      background: bg,
      // Inga borders - vi ritar textur ovanpå
      imageRendering: 'pixelated'
    }}>
      {/* Grästuva-textur */}
      {(type === 'grassA' || type === 'grassB' || type === 'grassDark') && ((gx * 3 + gy * 5) % 4 === 0) && (
        <svg viewBox="0 0 40 40" style={{ position: 'absolute', inset: 0 }}>
          <rect x="6" y="26" width="3" height="5" fill={PAL.grassDark} />
          <rect x="9" y="28" width="2" height="3" fill={PAL.grassDark} />
          <rect x="26" y="14" width="3" height="5" fill={PAL.grassDark} />
          <rect x="29" y="16" width="2" height="3" fill={PAL.grassDark} />
        </svg>
      )}
      {/* Path-textur - små prickar */}
      {type === 'path' && ((gx * 5 + gy * 3) % 3 === 0) && (
        <svg viewBox="0 0 40 40" style={{ position: 'absolute', inset: 0 }}>
          <rect x="12" y="16" width="3" height="3" fill={PAL.pathDark} />
          <rect x="24" y="28" width="3" height="3" fill={PAL.pathDark} />
        </svg>
      )}
      {/* Vatten - vågor */}
      {type === 'water' && (
        <svg viewBox="0 0 40 40" style={{ position: 'absolute', inset: 0 }}>
          <rect x="0" y="0" width="40" height="40" fill={PAL.water} />
          <rect x="6" y="10" width="6" height="2" fill={PAL.waterDark} />
          <rect x="20" y="18" width="8" height="2" fill={PAL.waterDark} />
          <rect x="4" y="28" width="6" height="2" fill={PAL.waterDark} />
          <rect x="24" y="32" width="6" height="2" fill={PAL.waterDark} />
        </svg>
      )}
    </div>
  );
}

// Träd
function Tree({ gx, gy }) {
  return (
    <div style={{
      position: 'absolute',
      left: gx * TILE - 4, top: gy * TILE - 18,
      width: TILE + 8, height: TILE + 18,
      zIndex: gy * 100 + 10
    }}>
      <svg viewBox="0 0 48 58" style={{ width: '100%', height: '100%' }} shapeRendering="crispEdges">
        {/* Skugga */}
        <ellipse cx="24" cy="54" rx="16" ry="3" fill="rgba(0,0,0,0.25)" />
        {/* Stam */}
        <rect x="20" y="38" width="8" height="14" fill={PAL.trunk} />
        <rect x="18" y="38" width="2" height="14" fill="#3a2010" />
        {/* Krona - Pokémon-stil med segment */}
        <circle cx="24" cy="22" r="18" fill={PAL.tree} />
        <circle cx="16" cy="16" r="10" fill={PAL.treeLight} />
        <circle cx="30" cy="14" r="8" fill={PAL.treeLight} />
        <circle cx="22" cy="26" r="6" fill={PAL.treeLight} />
        {/* Svart outline */}
        <circle cx="24" cy="22" r="18" fill="none" stroke="#1a2010" strokeWidth="1.5" />
        <rect x="20" y="38" width="8" height="14" fill="none" stroke="#1a2010" strokeWidth="1.5" />
      </svg>
    </div>
  );
}

// Blomma
function Flower({ gx, gy, color }) {
  const petal = color === 'pink' ? PAL.flower : PAL.flowerY;
  return (
    <div style={{
      position: 'absolute',
      left: gx * TILE + 10, top: gy * TILE + 14,
      width: 20, height: 20, zIndex: gy * 100 + 5
    }}>
      <svg viewBox="0 0 20 20" shapeRendering="crispEdges">
        <rect x="9" y="14" width="2" height="5" fill={PAL.grassDark} />
        <rect x="8" y="8" width="4" height="3" fill={petal} />
        <rect x="5" y="10" width="3" height="2" fill={petal} />
        <rect x="12" y="10" width="3" height="2" fill={petal} />
        <rect x="8" y="5" width="4" height="2" fill={petal} />
        <rect x="9" y="9" width="2" height="2" fill={color === 'pink' ? PAL.flowerY : '#ff8030'} />
      </svg>
    </div>
  );
}

// Sten
function Rock({ gx, gy }) {
  return (
    <div style={{
      position: 'absolute',
      left: gx * TILE + 4, top: gy * TILE + 8,
      width: 32, height: 28, zIndex: gy * 100 + 5
    }}>
      <svg viewBox="0 0 32 28" shapeRendering="crispEdges">
        <ellipse cx="16" cy="24" rx="12" ry="3" fill="rgba(0,0,0,0.25)" />
        <path d="M 4 20 L 6 8 L 12 4 L 22 6 L 28 14 L 26 22 L 6 22 Z" fill="#9098a0" stroke="#2a2a2a" strokeWidth="1.5" />
        <path d="M 8 12 L 12 8 L 16 10 L 14 14 Z" fill="#b0b8c0" />
      </svg>
    </div>
  );
}

// Skylt
function Sign({ gx, gy, label }) {
  const lines = label.split('\n');
  return (
    <div style={{
      position: 'absolute',
      left: gx * TILE, top: gy * TILE - 20,
      width: TILE, height: TILE + 20, zIndex: gy * 100 + 8
    }}>
      <svg viewBox="0 0 40 60" shapeRendering="crispEdges">
        <ellipse cx="20" cy="57" rx="12" ry="2" fill="rgba(0,0,0,0.25)" />
        {/* Stolpe */}
        <rect x="18" y="34" width="4" height="22" fill={PAL.trunk} stroke="#1a1a1a" strokeWidth="1" />
        {/* Skylt */}
        <rect x="4" y="10" width="32" height="26" fill="#c89a58" stroke="#1a1a1a" strokeWidth="2" />
        <rect x="6" y="12" width="28" height="22" fill="#e8b868" />
        <text x="20" y="20" textAnchor="middle" fill="#1a1a1a" fontSize="6" fontFamily="ui-monospace, Menlo, monospace" fontWeight="700">{lines[0]}</text>
        {lines[1] && <text x="20" y="28" textAnchor="middle" fill="#1a1a1a" fontSize="6" fontFamily="ui-monospace, Menlo, monospace" fontWeight="700">{lines[1]}</text>}
      </svg>
    </div>
  );
}

// Fotbollsplan - top-down, Pokémon-stil
function Pitch({ data, completed, highlighted }) {
  const w = data.w * TILE;
  const h = data.h * TILE;
  return (
    <div style={{
      position: 'absolute',
      left: (data.gx - Math.floor(data.w/2)) * TILE,
      top: (data.gy - Math.floor(data.h/2)) * TILE,
      width: w, height: h,
      zIndex: data.gy * 100 - 1
    }}>
      <svg viewBox={`0 0 ${w} ${h}`} shapeRendering="crispEdges" style={{ display: 'block' }}>
        {/* Plan-kantsten runt om */}
        <rect x="0" y="0" width={w} height={h} fill="#8a6a3a" />
        <rect x="4" y="4" width={w-8} height={h-8} fill={data.color} stroke="#1a1a1a" strokeWidth="2" />
        {/* Gräsmönster - ränder */}
        {Array.from({length: Math.floor((w-8)/16)}).map((_, i) => (
          <rect key={i} x={4 + i*16} y="4" width="8" height={h-8}
            fill={i % 2 === 0 ? 'rgba(255,255,255,0.08)' : 'transparent'} />
        ))}
        {/* Ytterlinje */}
        <rect x="10" y="10" width={w-20} height={h-20} fill="none" stroke="#fff" strokeWidth="2" />
        {/* Mittlinje */}
        <line x1={w/2} y1="10" x2={w/2} y2={h-10} stroke="#fff" strokeWidth="2" />
        {/* Mittcirkel */}
        <circle cx={w/2} cy={h/2} r="14" fill="none" stroke="#fff" strokeWidth="2" />
        <circle cx={w/2} cy={h/2} r="2" fill="#fff" />
        {/* Mål vänster */}
        <rect x="6" y={h/2 - 10} width="6" height="20" fill="#fff" stroke="#1a1a1a" strokeWidth="1" />
        <rect x="10" y={h/2 - 14} width="14" height="28" fill="none" stroke="#fff" strokeWidth="1.5" />
        {/* Mål höger */}
        <rect x={w-12} y={h/2 - 10} width="6" height="20" fill="#fff" stroke="#1a1a1a" strokeWidth="1" />
        <rect x={w-24} y={h/2 - 14} width="14" height="28" fill="none" stroke="#fff" strokeWidth="1.5" />
        {/* Completed-indikator */}
        {completed && (
          <g>
            <circle cx={w/2} cy={h/2} r="12" fill="#ffd428" stroke="#1a1a1a" strokeWidth="2" />
            <text x={w/2} y={h/2 + 5} textAnchor="middle" fontSize="14" fontWeight="700" fill="#1a1a1a">★</text>
          </g>
        )}
      </svg>
    </div>
  );
}

// Lagflagga vid plan
// Flagg-animation: Quadratic Bezier cloth-sim (inspirerad av r3mainer, CC BY-SA 4.0)
// Amplituden ökar linjärt från stången (x=0) till fria änden (x=1).
const FLAG_W = 44;
const FLAG_H = 40;
const FLAG_AMP = 6;
const FLAG_NFRAMES = 14;

const FLAG_ANIM_VALUES = (() => {
  const freq = 1.0;
  const paths = [];
  for (let i = 0; i <= FLAG_NFRAMES; i++) {
    const t = i / FLAG_NFRAMES;
    let s = 'M0 0', bx = 0, by = 0;
    // Övre kant: stång → fri ände
    for (let xi = 1; xi <= 10; xi++) {
      const cx = xi * 0.1 - 0.05;
      const ex = xi * 0.1;
      const ay = FLAG_AMP * cx * Math.sin((cx * freq - t) * 2 * Math.PI);
      const ey = FLAG_AMP * ex * Math.sin((ex * freq - t) * 2 * Math.PI);
      s += `Q${(cx * FLAG_W).toFixed(1)} ${ay.toFixed(1)} ${(ex * FLAG_W).toFixed(1)} ${ey.toFixed(1)}`;
      bx = ex * FLAG_W; by = ey;
    }
    // Höger kant: rakt ner
    s += `L${bx.toFixed(1)} ${(by + FLAG_H).toFixed(1)}`;
    // Nedre kant: fri ände → stång
    for (let xi = 9; xi >= 0; xi--) {
      const cx = xi * 0.1 + 0.05;
      const ex = xi * 0.1;
      const ay = FLAG_AMP * cx * Math.sin((cx * freq - t) * 2 * Math.PI) + FLAG_H;
      const ey = FLAG_AMP * ex * Math.sin((ex * freq - t) * 2 * Math.PI) + FLAG_H;
      s += `Q${(cx * FLAG_W).toFixed(1)} ${ay.toFixed(1)} ${(ex * FLAG_W).toFixed(1)} ${ey.toFixed(1)}`;
    }
    s += 'Z';
    paths.push(s);
  }
  paths.push(paths[0]); // sömlös loop
  return paths.join(';');
})();

const FLAG_PATH_INIT = FLAG_ANIM_VALUES.split(';')[0];
const FLAG_DELAYS = { parken: '0s', kullen: '-0.7s', arena: '-1.3s', stadion: '-0.4s' };
const FLAG_PAD = FLAG_AMP + 2;

function PitchFlag({ pitch, matchData }) {
  if (!matchData || !matchData.team) return null;
  const x = (pitch.gx + Math.floor(pitch.w / 2)) * TILE + 4;
  const y = (pitch.gy - Math.floor(pitch.h / 2)) * TILE - 38;
  const delay = FLAG_DELAYS[pitch.id] || '0s';
  const clipId = `flagclip-${pitch.id}`;
  return (
    <div style={{
      position: 'absolute', left: x, top: y,
      zIndex: pitch.gy * 100 + 15, pointerEvents: 'none'
    }}>
      {/* Stång */}
      <div style={{
        position: 'absolute', left: 2, top: 2,
        width: 4, height: FLAG_H + FLAG_PAD * 2 + 48,
        background: 'linear-gradient(90deg, #5a4424 0%, #8a6a3e 50%, #5a4424 100%)',
        outline: '1px solid #1a1a1a'
      }} />
      {/* Stångknopp */}
      <div style={{
        position: 'absolute', left: 0, top: -2,
        width: 8, height: 8, borderRadius: '50%',
        background: '#d4b04e', border: '1px solid #1a1a1a'
      }} />
      {/* Vajande SVG-flagga */}
      <svg
        style={{ position: 'absolute', left: 6, top: 0, overflow: 'visible' }}
        width={FLAG_W} height={FLAG_H + FLAG_PAD * 2}
        viewBox={`0 -${FLAG_PAD} ${FLAG_W} ${FLAG_H + FLAG_PAD * 2}`}
        shapeRendering="geometricPrecision"
      >
        <defs>
          <clipPath id={clipId}>
            <path d={FLAG_PATH_INIT}>
              <animate attributeName="d" dur="2.2s" repeatCount="indefinite"
                values={FLAG_ANIM_VALUES} begin={delay} calcMode="linear" />
            </path>
          </clipPath>
        </defs>
        {/* Vit flaggbakgrund (vajar med samma path) */}
        <path fill="#fdfcf7" stroke="#1a1a1a" strokeWidth="1.5" strokeLinejoin="round" d={FLAG_PATH_INIT}>
          <animate attributeName="d" dur="2.2s" repeatCount="indefinite"
            values={FLAG_ANIM_VALUES} begin={delay} calcMode="linear" />
        </path>
        {/* Logo klippt mot flagg-formen */}
        <image
          href={`data/teams/${matchData.team}/logo.svg`}
          x="2" y="2" width={FLAG_W - 4} height={FLAG_H - 4}
          clipPath={`url(#${clipId})`}
          preserveAspectRatio="xMidYMid meet"
        />
      </svg>
    </div>
  );
}

// SPELARE - top-down sprite, Pokémon-inspirerad
function Player({ px, py, dir, moving, step }) {
  // px, py i pixel-koordinater
  const facing = dir || 'down';
  const walkOffset = moving ? (step ? 1 : -1) : 0;

  return (
    <div style={{
      position: 'absolute',
      left: px - TILE/2, top: py - TILE + 4,
      width: TILE, height: TILE + 4,
      zIndex: Math.floor(py / TILE) * 100 + 50,
      pointerEvents: 'none',
      imageRendering: 'pixelated'
    }}>
      <svg viewBox="0 0 32 36" shapeRendering="crispEdges">
        {/* Skugga */}
        <ellipse cx="16" cy="34" rx="10" ry="2" fill="rgba(0,0,0,0.35)" />

        {facing === 'down' && (
          <g>
            {/* Ben */}
            <rect x="11" y="26" width="4" height="6" fill="#1a3a7a" />
            <rect x="17" y="26" width="4" height="6" fill="#1a3a7a" />
            <rect x={walkOffset > 0 ? 11 : 17} y="31" width="4" height="2" fill="#1a1a1a" />
            {/* Kropp - tröja */}
            <rect x="8" y="16" width="16" height="11" fill="#e84820" />
            <rect x="8" y="16" width="16" height="2" fill="#fff" />
            {/* Armar */}
            <rect x="6" y="17" width="3" height="8" fill="#f4c896" />
            <rect x="23" y="17" width="3" height="8" fill="#f4c896" />
            {/* Huvud */}
            <rect x="9" y="6" width="14" height="11" fill="#f4c896" />
            {/* Hår */}
            <rect x="9" y="4" width="14" height="4" fill="#5a3a1a" />
            <rect x="8" y="6" width="1" height="4" fill="#5a3a1a" />
            <rect x="23" y="6" width="1" height="4" fill="#5a3a1a" />
            {/* Ögon */}
            <rect x="12" y="11" width="2" height="2" fill="#1a1a1a" />
            <rect x="18" y="11" width="2" height="2" fill="#1a1a1a" />
            {/* Outline */}
            <path d="M 9 4 L 23 4 L 24 6 L 24 16 L 26 17 L 26 25 L 23 25 L 21 32 L 11 32 L 9 25 L 6 25 L 6 17 L 8 16 L 8 6 Z"
              fill="none" stroke="#1a1a1a" strokeWidth="1" />
          </g>
        )}

        {facing === 'up' && (
          <g>
            {/* Ben */}
            <rect x="11" y="26" width="4" height="6" fill="#1a3a7a" />
            <rect x="17" y="26" width="4" height="6" fill="#1a3a7a" />
            {/* Kropp - tröja (baksida) */}
            <rect x="8" y="16" width="16" height="11" fill="#e84820" />
            {/* Armar */}
            <rect x="6" y="17" width="3" height="8" fill="#f4c896" />
            <rect x="23" y="17" width="3" height="8" fill="#f4c896" />
            {/* Nummer på ryggen */}
            <text x="16" y="24" textAnchor="middle" fontSize="7" fill="#fff" fontWeight="700" fontFamily="ui-monospace, monospace">9</text>
            {/* Huvud (baksida - bara hår) */}
            <rect x="9" y="4" width="14" height="13" fill="#5a3a1a" />
            <rect x="9" y="14" width="14" height="3" fill="#f4c896" />
            <path d="M 9 4 L 23 4 L 24 6 L 24 16 L 26 17 L 26 25 L 23 25 L 21 32 L 11 32 L 9 25 L 6 25 L 6 17 L 8 16 L 8 6 Z"
              fill="none" stroke="#1a1a1a" strokeWidth="1" />
          </g>
        )}

        {(facing === 'left' || facing === 'right') && (
          <g transform={facing === 'left' ? 'scale(-1,1) translate(-32,0)' : ''}>
            {/* Ben */}
            <rect x="12" y="26" width="5" height="6" fill="#1a3a7a" />
            <rect x={walkOffset > 0 ? 11 : 13} y="31" width="5" height="2" fill="#1a1a1a" />
            {/* Kropp */}
            <rect x="9" y="16" width="14" height="11" fill="#e84820" />
            <rect x="9" y="16" width="14" height="2" fill="#fff" />
            {/* Arm fram */}
            <rect x="18" y="18" width="3" height="7" fill="#f4c896" />
            {/* Huvud */}
            <rect x="10" y="6" width="12" height="11" fill="#f4c896" />
            {/* Hår */}
            <rect x="10" y="4" width="12" height="4" fill="#5a3a1a" />
            <rect x="10" y="6" width="3" height="4" fill="#5a3a1a" />
            {/* Öga */}
            <rect x="17" y="11" width="2" height="2" fill="#1a1a1a" />
            {/* Näsa */}
            <rect x="20" y="12" width="1" height="2" fill="#d4a076" />
            <path d="M 10 4 L 22 4 L 23 6 L 23 16 L 24 17 L 24 25 L 21 25 L 19 32 L 12 32 L 10 25 L 8 25 L 8 17 L 9 16 L 9 6 Z"
              fill="none" stroke="#1a1a1a" strokeWidth="1" />
          </g>
        )}
      </svg>
    </div>
  );
}

// Dialogruta i Pokémon-stil
function DialogBox({ children }) {
  return (
    <div style={{
      position: 'absolute',
      left: '50%', bottom: 20,
      transform: 'translateX(-50%)',
      width: 'min(520px, 92%)',
      zIndex: 100,
      pointerEvents: 'none'
    }}>
      <div style={{
        background: '#fdfcf7',
        border: '4px solid #1a1a1a',
        borderRadius: 8,
        boxShadow: 'inset 0 0 0 2px #fdfcf7, inset 0 0 0 4px #3464a8, 0 4px 0 rgba(0,0,0,0.3)',
        padding: '16px 20px',
        fontFamily: 'ui-monospace, Menlo, monospace',
        fontSize: 13,
        color: '#1a1a1a',
        lineHeight: 1.5
      }}>
        {children}
      </div>
    </div>
  );
}

// ── NPC-system ─────────────────────────────────────────────────────────────

const NPC_SPEED = 1.1;
const TACKLE_DURATION = 540; // ~9s vid 60fps — orimligt länge

const NPC_CONFIGS = [
  { id: 1, gx: 8,  gy: 5,  color: '#3464a8', hairColor: '#5a3a1a', num: '7'  },
  { id: 2, gx: 20, gy: 8,  color: '#c82828', hairColor: '#1a1a1a', num: '11' },
  { id: 3, gx: 10, gy: 14, color: '#1a7a3a', hairColor: '#f4c896', num: '3'  },
  { id: 4, gx: 18, gy: 5,  color: '#8b4513', hairColor: '#ffd700', num: '9'  },
  { id: 5, gx: 24, gy: 12, color: '#6a0dad', hairColor: '#5a3a1a', num: '6'  },
  { id: 6, gx: 7,  gy: 12, color: '#1a6a6a', hairColor: '#1a1a1a', num: '2'  },
];

function initNPC(cfg) {
  return {
    id: cfg.id,
    px: cfg.gx * TILE, py: cfg.gy * TILE,
    dir: 'down', moving: false, step: false, stepCounter: 0,
    state: 'walking', tackledTimer: 0,
    color: cfg.color, hairColor: cfg.hairColor, num: cfg.num,
    homeGx: cfg.gx, homeGy: cfg.gy,
    targetPx: cfg.gx * TILE, targetPy: cfg.gy * TILE,
    wanderCooldown: Math.floor(Math.random() * 120),
  };
}

function pickNewTarget(npc, worldMap) {
  // Välj ett mål i en båge framåt (bias mot nuvarande riktning) för naturligare rörelse
  const angle = npc.lastAngle !== undefined ? npc.lastAngle : Math.random() * Math.PI * 2;
  const spread = Math.PI * 0.75; // ±135° svängradie
  for (let i = 0; i < 14; i++) {
    const a = angle + (Math.random() - 0.5) * spread;
    const dist = 4 + Math.random() * 7;
    const tGx = Math.max(2, Math.min(WORLD_W - 2, Math.round(npc.homeGx + Math.cos(a) * dist)));
    const tGy = Math.max(2, Math.min(WORLD_H - 2, Math.round(npc.homeGy + Math.sin(a) * dist)));
    if (!isBlocked(worldMap, tGx, tGy, DECORATIONS, [])) {
      return { ...npc, targetPx: tGx * TILE, targetPy: tGy * TILE, lastAngle: a };
    }
  }
  // Fallback: gå mot hemposition
  return { ...npc, targetPx: npc.homeGx * TILE, targetPy: npc.homeGy * TILE };
}

function updateNPC(npc, player, worldMap) {
  if (npc.state === 'tackled') {
    const t = npc.tackledTimer - 1;
    if (t <= 0) return { ...npc, state: 'walking', tackledTimer: 0, wanderCooldown: 80 };
    return { ...npc, tackledTimer: t };
  }

  // Kollision med spelare
  if (player.moving) {
    const dx = npc.px - player.px, dy = npc.py - player.py;
    if (dx * dx + dy * dy < (TILE * 0.72) * (TILE * 0.72)) {
      return { ...npc, state: 'tackled', tackledTimer: TACKLE_DURATION, moving: false };
    }
  }

  let { px, py, targetPx, targetPy, wanderCooldown, step, stepCounter } = npc;

  if (wanderCooldown > 0) return { ...npc, wanderCooldown: wanderCooldown - 1, moving: false };

  const tdx = targetPx - px, tdy = targetPy - py;
  const tdist = Math.sqrt(tdx * tdx + tdy * tdy);

  if (tdist < NPC_SPEED + 1) {
    return pickNewTarget({ ...npc, px: targetPx, py: targetPy, moving: false,
      wanderCooldown: 20 + Math.floor(Math.random() * 60) }, worldMap);
  }

  const nx = tdx / tdist, ny = tdy / tdist;
  let newPx = px + nx * NPC_SPEED, newPy = py + ny * NPC_SPEED;
  const gx = Math.floor(newPx / TILE), gy = Math.floor(newPy / TILE);

  if (isBlocked(worldMap, gx, gy, DECORATIONS, [])) {
    const gxOnly = Math.floor((px + nx * NPC_SPEED) / TILE);
    const gyOnly = Math.floor((py + ny * NPC_SPEED) / TILE);
    if (!isBlocked(worldMap, gxOnly, Math.floor(py / TILE), DECORATIONS, [])) {
      newPy = py;
    } else if (!isBlocked(worldMap, Math.floor(px / TILE), gyOnly, DECORATIONS, [])) {
      newPx = px;
    } else {
      return pickNewTarget({ ...npc, moving: false }, worldMap);
    }
  }

  newPx = Math.max(TILE, Math.min((WORLD_W - 1) * TILE, newPx));
  newPy = Math.max(TILE, Math.min((WORLD_H - 1) * TILE, newPy));

  const newDir = Math.abs(tdx) > Math.abs(tdy) ? (tdx > 0 ? 'right' : 'left') : (tdy > 0 ? 'down' : 'up');
  const newStepCounter = stepCounter + 1;
  const newStep = newStepCounter % 14 === 0 ? !step : step;

  return { ...npc, px: newPx, py: newPy, dir: newDir, moving: true, step: newStep, stepCounter: newStepCounter };
}

function NPCSprite({ px, py, dir, moving, step, state, tackledTimer, color, hairColor, num }) {
  const walkOffset = moving ? (step ? 1 : -1) : 0;
  const facing = dir || 'down';

  if (state === 'tackled') {
    const starBlink = Math.floor(tackledTimer / 18) % 2 === 0;
    return (
      <div style={{
        position: 'absolute',
        left: px - TILE * 1.1, top: py - TILE * 0.6,
        width: TILE * 2.6, height: TILE * 1.2,
        zIndex: Math.floor(py / TILE) * 100 + 49,
        pointerEvents: 'none', imageRendering: 'pixelated'
      }}>
        <svg viewBox="0 0 104 48" shapeRendering="crispEdges">
          {/* Skugga */}
          <ellipse cx="40" cy="44" rx="34" ry="4" fill="rgba(0,0,0,0.28)" />
          {/* Kropp liggande */}
          <rect x="14" y="24" width="34" height="10" fill={color} stroke="#1a1a1a" strokeWidth="1.5" />
          <rect x="14" y="24" width="34" height="2" fill="rgba(255,255,255,0.35)" />
          {/* Huvud */}
          <rect x="3" y="22" width="13" height="13" fill="#f4c896" stroke="#1a1a1a" strokeWidth="1.5" />
          <rect x="3" y="22" width="13" height="4" fill={hairColor} />
          {/* Smärtans ansikte — × ögon och krökt mun */}
          <rect x="6" y="28" width="2" height="2" fill="#1a1a1a" />
          <rect x="11" y="28" width="2" height="2" fill="#1a1a1a" />
          <rect x="7" y="27" width="2" height="1" fill="#1a1a1a" transform="rotate(45,8,27.5)" />
          <rect x="11" y="27" width="2" height="1" fill="#1a1a1a" transform="rotate(-45,12,27.5)" />
          <rect x="7" y="32" width="5" height="2" fill="#c84820" />
          {/* Ben */}
          <rect x="48" y="22" width="12" height="5" fill="#1a3a7a" stroke="#1a1a1a" strokeWidth="1" />
          <rect x="48" y="29" width="12" height="5" fill="#1a3a7a" stroke="#1a1a1a" strokeWidth="1" />
          {/* Upplyft böjt knä */}
          <rect x="58" y="15" width="5" height="14" fill="#1a3a7a" stroke="#1a1a1a" strokeWidth="1" />
          <rect x="58" y="15" width="9" height="4" fill="#1a3a7a" stroke="#1a1a1a" strokeWidth="1" />
          {/* Arm sträckt mot knät */}
          <rect x="28" y="17" width="4" height="10" fill="#f4c896" stroke="#1a1a1a" strokeWidth="1" />
          <rect x="32" y="15" width="26" height="4" fill="#f4c896" stroke="#1a1a1a" strokeWidth="1" />
          {/* Smärtstjärnor */}
          {starBlink && (
            <g>
              <text x="2" y="16" fontSize="10" fill="#ffd43b" fontFamily="serif">★</text>
              <text x="14" y="11" fontSize="7" fill="#ff4757" fontFamily="sans-serif">!</text>
              <text x="22" y="9" fontSize="7" fill="#ff4757" fontFamily="sans-serif">!</text>
            </g>
          )}
          {!starBlink && (
            <g>
              <text x="4" y="14" fontSize="8" fill="#ff6b35" fontFamily="serif">✦</text>
              <text x="16" y="10" fontSize="6" fill="#ffd43b" fontFamily="sans-serif">*</text>
            </g>
          )}
        </svg>
      </div>
    );
  }

  return (
    <div style={{
      position: 'absolute',
      left: px - TILE / 2, top: py - TILE + 4,
      width: TILE, height: TILE + 4,
      zIndex: Math.floor(py / TILE) * 100 + 49,
      pointerEvents: 'none', imageRendering: 'pixelated'
    }}>
      <svg viewBox="0 0 32 36" shapeRendering="crispEdges">
        <ellipse cx="16" cy="34" rx="10" ry="2" fill="rgba(0,0,0,0.28)" />
        {facing === 'down' && (
          <g>
            <rect x="11" y="26" width="4" height="6" fill="#1a3a7a" />
            <rect x="17" y="26" width="4" height="6" fill="#1a3a7a" />
            <rect x={walkOffset > 0 ? 11 : 17} y="31" width="4" height="2" fill="#1a1a1a" />
            <rect x="8" y="16" width="16" height="11" fill={color} />
            <rect x="8" y="16" width="16" height="2" fill="rgba(255,255,255,0.35)" />
            <rect x="6" y="17" width="3" height="8" fill="#f4c896" />
            <rect x="23" y="17" width="3" height="8" fill="#f4c896" />
            <rect x="9" y="6" width="14" height="11" fill="#f4c896" />
            <rect x="9" y="4" width="14" height="4" fill={hairColor} />
            <rect x="8" y="6" width="1" height="4" fill={hairColor} />
            <rect x="23" y="6" width="1" height="4" fill={hairColor} />
            <rect x="12" y="11" width="2" height="2" fill="#1a1a1a" />
            <rect x="18" y="11" width="2" height="2" fill="#1a1a1a" />
            <path d="M 9 4 L 23 4 L 24 6 L 24 16 L 26 17 L 26 25 L 23 25 L 21 32 L 11 32 L 9 25 L 6 25 L 6 17 L 8 16 L 8 6 Z"
              fill="none" stroke="#1a1a1a" strokeWidth="1" />
          </g>
        )}
        {facing === 'up' && (
          <g>
            <rect x="11" y="26" width="4" height="6" fill="#1a3a7a" />
            <rect x="17" y="26" width="4" height="6" fill="#1a3a7a" />
            <rect x="8" y="16" width="16" height="11" fill={color} />
            <rect x="6" y="17" width="3" height="8" fill="#f4c896" />
            <rect x="23" y="17" width="3" height="8" fill="#f4c896" />
            <text x="16" y="24" textAnchor="middle" fontSize="7" fill="rgba(255,255,255,0.8)" fontWeight="700" fontFamily="ui-monospace,monospace">{num}</text>
            <rect x="9" y="4" width="14" height="13" fill={hairColor} />
            <rect x="9" y="14" width="14" height="3" fill="#f4c896" />
            <path d="M 9 4 L 23 4 L 24 6 L 24 16 L 26 17 L 26 25 L 23 25 L 21 32 L 11 32 L 9 25 L 6 25 L 6 17 L 8 16 L 8 6 Z"
              fill="none" stroke="#1a1a1a" strokeWidth="1" />
          </g>
        )}
        {(facing === 'left' || facing === 'right') && (
          <g transform={facing === 'left' ? 'scale(-1,1) translate(-32,0)' : ''}>
            <rect x="12" y="26" width="5" height="6" fill="#1a3a7a" />
            <rect x={walkOffset > 0 ? 11 : 13} y="31" width="5" height="2" fill="#1a1a1a" />
            <rect x="9" y="16" width="14" height="11" fill={color} />
            <rect x="9" y="16" width="14" height="2" fill="rgba(255,255,255,0.35)" />
            <rect x="18" y="18" width="3" height="7" fill="#f4c896" />
            <rect x="10" y="6" width="12" height="11" fill="#f4c896" />
            <rect x="10" y="4" width="12" height="4" fill={hairColor} />
            <rect x="10" y="6" width="3" height="4" fill={hairColor} />
            <rect x="17" y="11" width="2" height="2" fill="#1a1a1a" />
            <rect x="20" y="12" width="1" height="2" fill="#d4a076" />
            <path d="M 10 4 L 22 4 L 23 6 L 23 16 L 24 17 L 24 25 L 21 25 L 19 32 L 12 32 L 10 25 L 8 25 L 8 17 L 9 16 L 9 6 Z"
              fill="none" stroke="#1a1a1a" strokeWidth="1" />
          </g>
        )}
      </svg>
    </div>
  );
}

// ── Slut NPC-system ────────────────────────────────────────────────────────

function IsoWorld({ player, onEnterPitch, onEnterSign, completedMatches }) {
  const [world] = useState(() => buildWorld());
  const [npcs, setNpcs] = useState(() => NPC_CONFIGS.map(initNPC));
  const playerRef = useRef(player);

  useEffect(() => { playerRef.current = player; }, [player]);

  useEffect(() => {
    let raf;
    const tick = () => {
      const p = playerRef.current;
      setNpcs(prev => prev.map(npc => updateNPC(npc, p, world)));
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [world]);

  // Kamera följer spelare (centrerad)
  const camX = player.px;
  const camY = player.py;

  // Rendera tiles
  const tiles = [];
  for (let y = 0; y < WORLD_H; y++) {
    for (let x = 0; x < WORLD_W; x++) {
      tiles.push(<Tile key={`${x}-${y}`} gx={x} gy={y} type={world[y][x]} />);
    }
  }

  // Kolla närmaste skylt och plan
  const playerGx = player.px / TILE;
  const playerGy = player.py / TILE;
  const nearbySign = SIGN_POSITIONS.find(s =>
    Math.abs(s.gx - playerGx) <= 1.2 && Math.abs(s.gy - playerGy) <= 1.2
  );
  const nearbyPitch = !nearbySign && PITCH_POSITIONS.find(p => {
    const dx = Math.abs(p.gx - playerGx);
    const dy = Math.abs(p.gy - playerGy);
    return dx <= (p.w/2 + 0.8) && dy <= (p.h/2 + 0.8);
  });

  return (
    <div style={{
      position: 'absolute', inset: 0, overflow: 'hidden',
      background: '#1a1a1a', imageRendering: 'pixelated'
    }}>
      {/* World container - scrollas via transform för att centrera spelaren */}
      <div style={{
        position: 'absolute',
        left: '50%', top: '50%',
        width: WORLD_W * TILE, height: WORLD_H * TILE,
        transform: `translate(${-camX}px, ${-camY}px)`,
        willChange: 'transform',
        imageRendering: 'pixelated'
      }}>
        {/* Bas-tiles */}
        {tiles}

        {/* Fotbollsplaner (ovanpå gräs) */}
        {PITCH_POSITIONS.map(p => {
          const data = window.MATCH_DATA.find(m => m.id === p.id);
          return (
            <Pitch
              key={p.id}
              data={{ ...p, color: data.color, name: data.name }}
              completed={completedMatches.includes(p.id)}
              highlighted={nearbyPitch && nearbyPitch.id === p.id}
            />
          );
        })}

        {/* Lagflaggor vid planer */}
        {PITCH_POSITIONS.map(p => (
          <PitchFlag key={`flag-${p.id}`} pitch={p} matchData={window.MATCH_DATA.find(m => m.id === p.id)} />
        ))}

        {/* Dekorationer */}
        {DECORATIONS.map((d, i) => {
          if (d.type === 'tree') return <Tree key={i} gx={d.gx} gy={d.gy} />;
          if (d.type === 'flower') return <Flower key={i} gx={d.gx} gy={d.gy} color={d.color} />;
          if (d.type === 'rock') return <Rock key={i} gx={d.gx} gy={d.gy} />;
          if (d.type === 'sign') return <Sign key={i} gx={d.gx} gy={d.gy} label={d.label} />;
          return null;
        })}

        {/* NPC:er */}
        {npcs.map(npc => (
          <NPCSprite
            key={npc.id}
            px={npc.px} py={npc.py}
            dir={npc.dir} moving={npc.moving} step={npc.step}
            state={npc.state} tackledTimer={npc.tackledTimer}
            color={npc.color} hairColor={npc.hairColor} num={npc.num}
          />
        ))}

        {/* Spelare */}
        <Player px={player.px} py={player.py} dir={player.dir} moving={player.moving} step={player.step} />
      </div>

      {/* Skylt-dialog — quiz */}
      {nearbySign && (
        <DialogBox>
          <div style={{ marginBottom: 6, color: '#8b4513', fontSize: 11, letterSpacing: '0.1em' }}>
            ▸ ANSLAGSTAVLA
          </div>
          <div style={{ fontFamily: 'Georgia, serif', fontSize: 15, marginBottom: 10 }}>
            <strong>{window.MATCH_DATA.find(m => m.id === nearbySign.id)?.name}</strong>
            <br />Svara på frågor om fotboll och taktik!
          </div>
          <button
            onClick={() => onEnterSign(nearbySign.id)}
            style={{
              pointerEvents: 'auto',
              background: '#c89a58',
              color: '#1a1a1a',
              border: '3px solid #1a1a1a',
              borderRadius: 4,
              padding: '8px 18px',
              fontFamily: 'ui-monospace, Menlo, monospace',
              fontSize: 12, fontWeight: 700, letterSpacing: '0.08em',
              cursor: 'pointer', boxShadow: '0 3px 0 #1a1a1a'
            }}
          >
            ▸ LÄSA TAVLAN [E]
          </button>
        </DialogBox>
      )}

      {/* Plan-dialog — fotbollsmatch */}
      {nearbyPitch && (
        <DialogBox>
          <div style={{ marginBottom: 6, color: '#3464a8', fontSize: 11, letterSpacing: '0.1em' }}>
            ▸ FOTBOLLSPLAN
          </div>
          <div style={{ fontFamily: 'Georgia, serif', fontSize: 15, marginBottom: 10 }}>
            <strong>{window.MATCH_DATA.find(m => m.id === nearbyPitch.id).name}</strong>
            <br />Spela 5 mot 5! WASD = rörelse, SPACE = passa, ENTER = skjut, Q+ENTER = megaskott
          </div>
          <button
            onClick={() => onEnterPitch(nearbyPitch.id)}
            style={{
              pointerEvents: 'auto',
              background: '#ffd428',
              color: '#1a1a1a',
              border: '3px solid #1a1a1a',
              borderRadius: 4,
              padding: '8px 18px',
              fontFamily: 'ui-monospace, Menlo, monospace',
              fontSize: 12, fontWeight: 700, letterSpacing: '0.08em',
              cursor: 'pointer', boxShadow: '0 3px 0 #1a1a1a'
            }}
          >
            ▸ SPELA MATCH [E]
          </button>
        </DialogBox>
      )}
    </div>
  );
}

const SIGN_POSITIONS = DECORATIONS.filter(d => d.type === 'sign' && d.id);

window.IsoWorld = IsoWorld;
window.PITCH_POSITIONS = PITCH_POSITIONS;
window.SIGN_POSITIONS = SIGN_POSITIONS;
window.WORLD_SIZE = WORLD_W;
window.WORLD_W = WORLD_W;
window.WORLD_H = WORLD_H;
window.TILE = TILE;
window.isBlocked = isBlocked;
window.buildWorld = buildWorld;
window.DECORATIONS = DECORATIONS;
