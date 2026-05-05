// ── 8-bit ljudmotor med Web Audio API ────────────────────────────────────────
// Alla ljud syntetiseras procedurellt — inga externa filer behövs.

(function() {
  let ctx = null;
  let crowdGain = null;
  let crowdOsc = null;
  let crowdRunning = false;
  let crowdLevel = 0;      // 0–1, interpoleras smidigt
  let crowdTarget = 0.04;  // viloläge
  let crowdRaf = null;

  function getCtx() {
    if (!ctx) {
      ctx = new (window.AudioContext || window.webkitAudioContext)();
    }
    if (ctx.state === 'suspended') ctx.resume();
    return ctx;
  }

  // ── Primitiver ──────────────────────────────────────────────────────────────

  function playTone(freq, type, dur, vol, startTime, detune) {
    const c = getCtx();
    const t = startTime || c.currentTime;
    const osc = c.createOscillator();
    const gain = c.createGain();
    osc.type = type || 'square';
    osc.frequency.setValueAtTime(freq, t);
    if (detune) osc.detune.setValueAtTime(detune, t);
    gain.gain.setValueAtTime(vol || 0.18, t);
    gain.gain.exponentialRampToValueAtTime(0.001, t + dur);
    osc.connect(gain);
    gain.connect(c.destination);
    osc.start(t);
    osc.stop(t + dur + 0.01);
  }

  function playNoise(dur, vol, startTime, filterFreq) {
    const c = getCtx();
    const t = startTime || c.currentTime;
    const bufLen = Math.ceil(c.sampleRate * dur);
    const buf = c.createBuffer(1, bufLen, c.sampleRate);
    const data = buf.getChannelData(0);
    for (let i = 0; i < bufLen; i++) data[i] = Math.random() * 2 - 1;
    const src = c.createBufferSource();
    src.buffer = buf;
    const filter = c.createBiquadFilter();
    filter.type = 'bandpass';
    filter.frequency.value = filterFreq || 800;
    filter.Q.value = 0.8;
    const gain = c.createGain();
    gain.gain.setValueAtTime(vol || 0.12, t);
    gain.gain.exponentialRampToValueAtTime(0.001, t + dur);
    src.connect(filter);
    filter.connect(gain);
    gain.connect(c.destination);
    src.start(t);
    src.stop(t + dur + 0.01);
  }

  // ── Ljud-recept ─────────────────────────────────────────────────────────────

  // Bollspark — snabb "thwack"
  function soundKick() {
    const c = getCtx();
    const t = c.currentTime;
    // Perkussiv nedgång
    playTone(180, 'sine', 0.06, 0.25, t);
    playTone(90,  'sine', 0.12, 0.18, t);
    playNoise(0.07, 0.1, t, 400);
  }

  // Hård spark / skott — kraftigare
  function soundShoot() {
    const c = getCtx();
    const t = c.currentTime;
    playTone(140, 'sine',   0.08, 0.3,  t);
    playTone(70,  'sine',   0.15, 0.22, t);
    playTone(220, 'square', 0.04, 0.08, t);
    playNoise(0.1, 0.15, t, 300);
  }

  // Visselpipa — kort retro-pipa
  function soundWhistle() {
    const c = getCtx();
    const t = c.currentTime;
    // Uppåtglidande ton
    const osc = c.createOscillator();
    const gain = c.createGain();
    osc.type = 'sine';
    osc.frequency.setValueAtTime(880, t);
    osc.frequency.linearRampToValueAtTime(1100, t + 0.08);
    osc.frequency.linearRampToValueAtTime(1050, t + 0.18);
    gain.gain.setValueAtTime(0.0, t);
    gain.gain.linearRampToValueAtTime(0.22, t + 0.02);
    gain.gain.setValueAtTime(0.22, t + 0.12);
    gain.gain.exponentialRampToValueAtTime(0.001, t + 0.22);
    osc.connect(gain);
    gain.connect(c.destination);
    osc.start(t);
    osc.stop(t + 0.25);
  }

  // Dubbel visselpipa — fulltid
  function soundWhistleFull() {
    const c = getCtx();
    soundWhistle();
    // Andra pip 0.3s senare
    const t2 = c.currentTime + 0.3;
    const osc = c.createOscillator();
    const gain = c.createGain();
    osc.type = 'sine';
    osc.frequency.setValueAtTime(880, t2);
    osc.frequency.linearRampToValueAtTime(1100, t2 + 0.08);
    osc.frequency.linearRampToValueAtTime(1050, t2 + 0.18);
    gain.gain.setValueAtTime(0.0, t2);
    gain.gain.linearRampToValueAtTime(0.22, t2 + 0.02);
    gain.gain.setValueAtTime(0.22, t2 + 0.12);
    gain.gain.exponentialRampToValueAtTime(0.001, t2 + 0.22);
    osc.connect(gain);
    gain.connect(c.destination);
    osc.start(t2);
    osc.stop(t2 + 0.25);
  }

  // Tackling/studs
  function soundTackle() {
    const c = getCtx();
    const t = c.currentTime;
    playTone(200, 'sawtooth', 0.05, 0.15, t);
    playNoise(0.08, 0.12, t, 600);
  }

  // Mål-jubel — episk 8-bit fanfar + crescendo-crowd
  function soundGoal() {
    const c = getCtx();
    const t = c.currentTime;
    // Fanfar-melodi (8-bit)
    const melody = [
      [523, 0.00], [659, 0.12], [784, 0.24], [1047,0.36],
      [784, 0.50], [1047,0.60], [1047,0.70], [1047,0.82]
    ];
    melody.forEach(([freq, dt]) => {
      playTone(freq, 'square', 0.1, 0.2, t + dt);
      playTone(freq * 0.5, 'square', 0.1, 0.08, t + dt); // oktav nere
    });
    // Arpeggio-ackord
    [523, 659, 784, 1047].forEach((f, i) => {
      playTone(f, 'square', 0.35, 0.12, t + 1.0 + i * 0.04);
    });
    // Crowd-jubel-swell
    setCrowdTarget(0.9);
    setTimeout(() => setCrowdTarget(0.25), 2500);
  }

  // Litet pip vid retur / inlägg
  function soundCorner() {
    const c = getCtx();
    const t = c.currentTime;
    playTone(660, 'square', 0.06, 0.12, t);
    playTone(880, 'square', 0.06, 0.12, t + 0.08);
  }

  // Mål-räddning (aah-sound)
  function soundSave() {
    const c = getCtx();
    const t = c.currentTime;
    [440, 392, 330].forEach((f, i) => {
      playTone(f, 'sine', 0.12, 0.15, t + i * 0.06);
    });
    playNoise(0.15, 0.08, t, 1200);
  }

  // ── Publikljud — kontinuerligt ambient ──────────────────────────────────────

  function startCrowd() {
    if (crowdRunning) return;
    const c = getCtx();
    crowdRunning = true;

    // Master gain för crowd
    crowdGain = c.createGain();
    crowdGain.gain.value = 0.0;
    crowdGain.connect(c.destination);

    // Tre oscillatorer med lite detune för ett "muller"-ljud
    const freqs = [80, 120, 160];
    freqs.forEach((f, i) => {
      const osc = c.createOscillator();
      osc.type = 'sawtooth';
      osc.frequency.value = f + Math.random() * 10;
      const filter = c.createBiquadFilter();
      filter.type = 'lowpass';
      filter.frequency.value = 400 + i * 80;
      filter.Q.value = 0.5;
      const g = c.createGain();
      g.gain.value = 0.3 + Math.random() * 0.2;
      osc.connect(filter);
      filter.connect(g);
      g.connect(crowdGain);
      osc.start();
    });

    // Brus-lager
    const bufLen = c.sampleRate * 4;
    const buf = c.createBuffer(1, bufLen, c.sampleRate);
    const data = buf.getChannelData(0);
    for (let i = 0; i < bufLen; i++) data[i] = Math.random() * 2 - 1;
    const noise = c.createBufferSource();
    noise.buffer = buf;
    noise.loop = true;
    const nFilter = c.createBiquadFilter();
    nFilter.type = 'bandpass';
    nFilter.frequency.value = 300;
    nFilter.Q.value = 0.4;
    const nGain = c.createGain();
    nGain.gain.value = 0.4;
    noise.connect(nFilter);
    nFilter.connect(nGain);
    nGain.connect(crowdGain);
    noise.start();

    // Smooth interpolation loop
    let last = c.currentTime;
    function smoothCrowd() {
      if (!crowdRunning) return;
      const now = c.currentTime;
      const dt = now - last;
      last = now;
      const speed = crowdLevel < crowdTarget ? 1.8 : 0.6;
      crowdLevel += (crowdTarget - crowdLevel) * Math.min(1, dt * speed);
      crowdGain.gain.setTargetAtTime(crowdLevel * 0.09, now, 0.05);
      crowdRaf = requestAnimationFrame(smoothCrowd);
    }
    smoothCrowd();
  }

  function stopCrowd() {
    crowdRunning = false;
    if (crowdRaf) cancelAnimationFrame(crowdRaf);
    if (crowdGain) {
      try { crowdGain.gain.setTargetAtTime(0, getCtx().currentTime, 0.3); } catch(e) {}
    }
  }

  function setCrowdTarget(v) {
    crowdTarget = Math.max(0, Math.min(1, v));
  }

  // Tension-buildup när bollen är nära mål
  function setCrowdTension(v) {
    // v = 0–1, 0 = vila, 1 = maxspänning
    const base = 0.04;
    const max = 0.35;
    setCrowdTarget(base + v * (max - base));
  }

  function stopAll() {
    stopCrowd();
    crowdLevel = 0;
    crowdTarget = 0.04;
    if (ctx) {
      try { ctx.suspend(); } catch(e) {}
    }
  }

  // ── Public API ───────────────────────────────────────────────────────────────
  window.SFX = {
    kick:        soundKick,
    shoot:       soundShoot,
    whistle:     soundWhistle,
    whistleFull: soundWhistleFull,
    tackle:      soundTackle,
    goal:        soundGoal,
    corner:      soundCorner,
    save:        soundSave,
    startCrowd,
    stopCrowd,
    stopAll,
    setCrowdTarget,
    setCrowdTension,
    unlock() { if (ctx) { try { ctx.resume(); } catch(e) {} } else { getCtx(); } startCrowd(); }
  };
})();
