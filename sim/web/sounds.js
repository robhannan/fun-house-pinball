// Sound effects for the Fun House, synthesised from scratch so there are no
// recordings to license. Loud and busy, to suit a 1950s pinball machine and a
// circus: clanging chime bars, bangs and thwacks, the knocker and the tilt
// buzzer, plus a full calliope band (oom-pah, drums, cymbals), brass fanfares,
// drum rolls, a clown horn, slide whistles and a sad trombone.
//
// Used in two places:
// - the simulator (sim.js) plays them with the Web Audio API
// - sim/render-sounds.mjs renders them to WAV files for the DFPlayer Mini's
//   SD card (`pnpm run render:sounds`)
//
// Keys are track numbers from `Sound::track()` in
// crates/pinball-core/src/sound.rs, and `durationMs` must match
// `Sound::duration_ms()`. A Rust test checks both. Pure functions only: no DOM,
// so Node can import this file.

const TAU = 2 * Math.PI;

/** MIDI note number to frequency in Hz. */
const hz = (note) => 440 * 2 ** ((note - 69) / 12);

/** Deterministic noise, so renders are identical every time. */
function noise(seed = 1) {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return (((t ^ (t >>> 14)) >>> 0) / 4294967296) * 2 - 1;
  };
}

/** A one-pole low-pass filter at `cutoff` Hz. */
function lowpass(sampleRate, cutoff) {
  const a = 1 - Math.exp((-TAU * cutoff) / sampleRate);
  let y = 0;
  return (x) => (y += a * (x - y));
}

/** A one-pole high-pass filter at `cutoff` Hz. */
function highpass(sampleRate, cutoff) {
  const low = lowpass(sampleRate, cutoff);
  return (x) => x - low(x);
}

/** Attack-release envelope: rises over `attack` s, falls over the last `release` s. */
function envelope(t, length, attack, release) {
  if (t < attack) return t / attack;
  if (t > length - release) return Math.max(0, (length - t) / release);
  return 1;
}

/** An empty buffer of `seconds`. */
const buffer = (sampleRate, seconds) => new Float32Array(Math.ceil(sampleRate * seconds));

/** Mixes `source` into `target`, starting at `at` seconds. */
function mix(target, source, sampleRate, at = 0, gain = 1) {
  const offset = Math.round(at * sampleRate);
  for (let i = 0; i < source.length && offset + i < target.length; i++) {
    target[offset + i] += source[i] * gain;
  }
  return target;
}

/**
 * Makes a sound loud: soft-clips it through `tanh` with `drive`, then scales it
 * to a peak just under full scale. More drive = denser and punchier.
 */
function master(samples, drive = 2) {
  let peak = 0;
  for (const s of samples) peak = Math.max(peak, Math.abs(s));
  if (peak === 0) return samples;
  let outPeak = 0;
  for (let i = 0; i < samples.length; i++) {
    samples[i] = Math.tanh((drive * samples[i]) / peak);
    outPeak = Math.max(outPeak, Math.abs(samples[i]));
  }
  for (let i = 0; i < samples.length; i++) samples[i] *= 0.97 / outPeak;
  return samples;
}

/** A sawtooth from a running phase in radians. */
const saw = (phase) => 2 * ((phase / TAU) % 1) - 1;

// ---------------------------------------------------------------------------
// Instruments

/** A clanging chime bar, struck hard: bright partials plus a metallic strike. */
function chime(sampleRate, freq, seconds, rand, decay = 5) {
  const out = buffer(sampleRate, seconds);
  const partials = [
    [1, 1],
    [2, 0.7],
    [3.01, 0.5],
    [4.17, 0.4],
    [5.43, 0.3],
    [6.8, 0.2],
  ];
  const strike = highpass(sampleRate, 3000);
  for (let i = 0; i < out.length; i++) {
    const t = i / sampleRate;
    let s = 0;
    for (const [ratio, gain] of partials) {
      s += gain * Math.sin(TAU * freq * ratio * t) * Math.exp(-t * decay * ratio ** 0.6);
    }
    out[i] = s * Math.min(1, t / 0.001) + 1.2 * strike(rand()) * Math.exp(-t * 200);
  }
  return out;
}

/** A calliope pipe: bright and breathy, two pipes slightly out of tune. */
function pipe(sampleRate, freq, seconds, rand) {
  const out = buffer(sampleRate, seconds);
  const hiss = highpass(sampleRate, 4000);
  let a = 0;
  let b = 0;
  for (let i = 0; i < out.length; i++) {
    const t = i / sampleRate;
    const wobble = 1 + 0.015 * Math.sin(TAU * 7 * t);
    a += (TAU * freq * wobble) / sampleRate;
    b += (TAU * freq * 1.004 * wobble) / sampleRate;
    let tone = 0;
    for (let h = 1; h <= 6; h++) tone += (Math.sin(h * a) + Math.sin(h * b)) / (h * 1.4);
    out[i] = (tone + 0.25 * hiss(rand())) * envelope(t, seconds, 0.008, 0.02);
  }
  return out;
}

/** A brass note: a bright sawtooth whose filter opens on the attack. */
function brass(sampleRate, freq, seconds, { vibrato = 0.006, bright = 3500 } = {}) {
  const out = buffer(sampleRate, seconds);
  let phase = 0;
  let y = 0;
  for (let i = 0; i < out.length; i++) {
    const t = i / sampleRate;
    phase += (TAU * freq * (1 + vibrato * Math.sin(TAU * 5.5 * t) * Math.min(1, t / 0.15))) / sampleRate;
    const cutoff = 300 + bright * Math.min(1, t / 0.04);
    y += (1 - Math.exp((-TAU * cutoff) / sampleRate)) * (saw(phase) - y);
    out[i] = y * envelope(t, seconds, 0.015, 0.05);
  }
  return out;
}

/** Drums. */
function kick(sampleRate, rand, seconds = 0.35) {
  const out = buffer(sampleRate, seconds);
  let phase = 0;
  for (let i = 0; i < out.length; i++) {
    const t = i / sampleRate;
    phase += (TAU * (45 + 160 * Math.exp(-t * 35))) / sampleRate;
    out[i] = Math.sin(phase) * Math.exp(-t * 8) + 0.5 * rand() * Math.exp(-t * 300);
  }
  return out;
}

function snare(sampleRate, rand, seconds = 0.16) {
  const out = buffer(sampleRate, seconds);
  const bright = highpass(sampleRate, 1500);
  for (let i = 0; i < out.length; i++) {
    const t = i / sampleRate;
    out[i] = bright(rand()) * Math.exp(-t * 25) + 0.5 * Math.sin(TAU * 190 * t) * Math.exp(-t * 40);
  }
  return out;
}

function cymbal(sampleRate, rand, seconds = 0.8) {
  const out = buffer(sampleRate, seconds);
  const bright = highpass(sampleRate, 5000);
  for (let i = 0; i < out.length; i++) {
    const t = i / sampleRate;
    // Ring modulation gives the metallic shimmer.
    out[i] = bright(rand()) * (1 + 0.5 * Math.sin(TAU * 540 * t)) * Math.exp(-t * 4);
  }
  return out;
}

/** A slide whistle gliding from `from` to `to` Hz, with a lively warble. */
function slideWhistle(sampleRate, from, to, seconds, rand, curve = 1) {
  const out = buffer(sampleRate, seconds);
  const breath = highpass(sampleRate, 3000);
  let phase = 0;
  for (let i = 0; i < out.length; i++) {
    const t = i / sampleRate;
    const progress = Math.min(1, t / (seconds - 0.05)) ** curve;
    const f = from * (to / from) ** progress * (1 + 0.04 * Math.sin(TAU * 9 * t));
    phase += (TAU * f) / sampleRate;
    out[i] = (Math.sin(phase) + 0.3 * Math.sin(2 * phase) + 0.1 * breath(rand())) * envelope(t, seconds, 0.01, 0.05);
  }
  return out;
}

// ---------------------------------------------------------------------------
// The sounds

function popBumper(sampleRate) {
  // BANG-clack: a punchy thud, a metallic clack and a short ring.
  const rand = noise(1);
  const out = buffer(sampleRate, 0.15);
  const clack = highpass(sampleRate, 2500);
  let phase = 0;
  for (let i = 0; i < out.length; i++) {
    const t = i / sampleRate;
    phase += (TAU * (55 + 350 * Math.exp(-t * 45))) / sampleRate;
    out[i] =
      1.2 * Math.sin(phase) * Math.exp(-t * 20) +
      1.0 * clack(rand()) * Math.exp(-t * 90) +
      0.4 * Math.sin(TAU * 1850 * t) * Math.exp(-t * 30);
  }
  return master(out, 3);
}

function kicker(sampleRate) {
  // THWACK: a sharp crack, a rubbery thump and a quick metal ping.
  const rand = noise(2);
  const out = buffer(sampleRate, 0.15);
  const crack = highpass(sampleRate, 1200);
  let phase = 0;
  for (let i = 0; i < out.length; i++) {
    const t = i / sampleRate;
    phase += (TAU * (120 + 200 * Math.exp(-t * 60))) / sampleRate;
    out[i] =
      1.4 * crack(rand()) * Math.exp(-t * 55) +
      0.9 * Math.sin(phase) * Math.exp(-t * 28) +
      0.4 * Math.sin(TAU * 2600 * t) * Math.exp(-t * 45);
  }
  return master(out, 4.5);
}

const lane = (sampleRate) => master(chime(sampleRate, hz(84), 0.4, noise(3), 6), 3);

function thirtyPoints(sampleRate) {
  // A fast ding-ding-DING up the chimes.
  const rand = noise(4);
  const out = buffer(sampleRate, 0.6);
  [84, 88, 91].forEach((note, n) => mix(out, chime(sampleRate, hz(note), 0.45, rand, 6), sampleRate, n * 0.07));
  mix(out, chime(sampleRate, hz(96), 0.4, rand, 7), sampleRate, 0.2, 0.7);
  return master(out, 3);
}

/** The bonus wheel's "bling" climbs a major scale as the wheel goes round. */
function bonusAdvance(step) {
  const scale = [0, 2, 4, 5, 7, 9, 11, 12, 14, 16];
  return (sampleRate) => {
    const rand = noise(10 + step);
    const note = 76 + scale[step];
    const out = buffer(sampleRate, 0.3);
    mix(out, chime(sampleRate, hz(note), 0.3, rand, 8), sampleRate, 0);
    mix(out, chime(sampleRate, hz(note + 12), 0.25, rand, 9), sampleRate, 0.05, 0.8);
    return master(out, 3);
  };
}

function bonusCollect(sampleRate) {
  // A snare roll building over a cymbal swell, then CRASH with a bass drum and
  // a brass chord stab.
  const rand = noise(5);
  const out = buffer(sampleRate, 2.0);
  for (let t = 0; t < 1.2; t += 0.024) {
    mix(out, snare(sampleRate, rand, 0.08), sampleRate, t, 0.55 + 0.7 * (t / 1.2) ** 2);
  }
  const swell = buffer(sampleRate, 1.2);
  const bright = highpass(sampleRate, 6000);
  for (let i = 0; i < swell.length; i++) swell[i] = bright(rand()) * (i / swell.length) ** 2;
  mix(out, swell, sampleRate, 0, 0.6);
  mix(out, kick(sampleRate, rand, 0.5), sampleRate, 1.2, 1.6);
  mix(out, cymbal(sampleRate, rand, 0.8), sampleRate, 1.2, 1.8);
  for (const note of [60, 64, 67, 72]) mix(out, brass(sampleRate, hz(note), 0.5), sampleRate, 1.2, 0.5);
  return master(out, 3.5);
}

function special(sampleRate) {
  // A brass fanfare: da-da-da-DAAA, with a snare roll and a cymbal crash.
  const rand = noise(6);
  const out = buffer(sampleRate, 1.5);
  const notes = [
    [67, 0, 0.11],
    [67, 0.12, 0.11],
    [67, 0.24, 0.11],
    [72, 0.36, 1.05],
  ];
  for (const [note, at, seconds] of notes) {
    mix(out, brass(sampleRate, hz(note), seconds, { vibrato: 0.012 }), sampleRate, at);
    mix(out, brass(sampleRate, hz(note + 4), seconds, { vibrato: 0.012 }), sampleRate, at, 0.6);
  }
  for (let t = 0; t < 0.36; t += 0.03) mix(out, snare(sampleRate, rand, 0.06), sampleRate, t, 0.4);
  mix(out, cymbal(sampleRate, rand, 1.1), sampleRate, 0.36, 1.2);
  mix(out, kick(sampleRate, rand), sampleRate, 0.36, 1.2);
  return master(out, 2.5);
}

function replay(sampleRate) {
  // The knocker: a huge BANG on the cabinet.
  const rand = noise(7);
  const out = buffer(sampleRate, 0.3);
  const crack = highpass(sampleRate, 800);
  let phase = 0;
  for (let i = 0; i < out.length; i++) {
    const t = i / sampleRate;
    phase += (TAU * (40 + 220 * Math.exp(-t * 50))) / sampleRate;
    out[i] =
      1.5 * Math.sin(phase) * Math.exp(-t * 12) +
      1.3 * crack(rand()) * Math.exp(-t * 70) +
      0.5 * Math.sin(TAU * 620 * t) * Math.exp(-t * 25);
  }
  return master(out, 4);
}

function tilt(sampleRate) {
  // The tilt buzzer: a harsh, pulsing alarm.
  const out = buffer(sampleRate, 1.2);
  let phase = 0;
  for (let i = 0; i < out.length; i++) {
    const t = i / sampleRate;
    phase += (TAU * 130) / sampleRate;
    const raw = Math.sign(Math.sin(phase)) + 0.7 * saw(phase * 1.01);
    const pulse = Math.sin(TAU * 9 * t) > -0.2 ? 1 : 0.15;
    out[i] = raw * pulse * envelope(t, 1.2, 0.005, 0.15);
  }
  return master(out, 3);
}

function gameStart(sampleRate) {
  // An original fast circus march for the full calliope band: melody, oom-pah
  // bass and chord stabs, bass drum and snare, cymbal crashes at each end.
  const rand = noise(8);
  const eighth = 0.105;
  const melody = [
    [67, 1], [66, 1], [67, 1], [68, 1], [69, 1], [70, 1], [71, 1], [72, 1],
    [76, 2], [72, 1], [76, 1], [79, 2], [76, 1], [79, 1],
    [84, 2], [83, 1], [81, 1], [79, 1], [77, 1], [76, 1], [74, 1],
    [72, 3], [null, 1], [84, 2],
  ];
  const beats = melody.reduce((sum, [, n]) => sum + n, 0);
  const out = buffer(sampleRate, 3.4);
  let at = 0;
  for (const [note, n] of melody) {
    if (note !== null) mix(out, pipe(sampleRate, hz(note), n * eighth * 0.9, rand), sampleRate, at);
    at += n * eighth;
  }
  // Bars of 8 eighths: C, C, G, C.
  const roots = [36, 36, 43, 36];
  for (let e = 0; e < beats - 2; e++) {
    const t = e * eighth;
    const root = roots[Math.floor(e / 8) % 4];
    if (e % 2 === 0) {
      mix(out, pipe(sampleRate, hz(e % 4 === 0 ? root : root + 7), eighth * 0.8, rand), sampleRate, t, 0.8);
      if (e % 4 === 0) mix(out, kick(sampleRate, rand, 0.25), sampleRate, t, 0.9);
    } else {
      // "Pah": a short chord stab, with the snare on the backbeat.
      for (const interval of [12, 16, 19]) mix(out, pipe(sampleRate, hz(root + interval), eighth * 0.5, rand), sampleRate, t, 0.25);
      if (e % 4 === 2 || e % 4 === 3) mix(out, snare(sampleRate, rand, 0.1), sampleRate, t, 0.35);
    }
  }
  mix(out, cymbal(sampleRate, rand, 0.9), sampleRate, 0, 0.8);
  const end = (beats - 2) * eighth;
  mix(out, cymbal(sampleRate, rand, 0.4), sampleRate, end, 1.2);
  mix(out, kick(sampleRate, rand, 0.3), sampleRate, end, 1.2);
  return master(out, 2.2);
}

function playerAdded(sampleRate) {
  // HONK HONK: a squeezed clown horn.
  const rand = noise(9);
  const out = buffer(sampleRate, 0.5);
  const honk = (pitch) => {
    const h = buffer(sampleRate, 0.19);
    const nasal = lowpass(sampleRate, 2600);
    let phase = 0;
    for (let i = 0; i < h.length; i++) {
      const t = i / sampleRate;
      phase += (TAU * pitch * (1 + 0.08 * Math.exp(-t * 30))) / sampleRate;
      h[i] = nasal(Math.tanh(5 * saw(phase)) + 0.15 * rand()) * envelope(t, 0.19, 0.005, 0.03);
    }
    return h;
  };
  mix(out, honk(440), sampleRate, 0);
  mix(out, honk(392), sampleRate, 0.24);
  return master(out, 3);
}

function ballServe(sampleRate) {
  // A cork "pop", then a quick slide whistle up.
  const rand = noise(10);
  const out = buffer(sampleRate, 0.7);
  mix(out, snare(sampleRate, rand, 0.05), sampleRate, 0, 0.8);
  mix(out, slideWhistle(sampleRate, 350, 2600, 0.62, rand, 1.6), sampleRate, 0.05);
  return master(out, 2.2);
}

function drain(sampleRate) {
  // A long slide whistle down, landing with a comic "bonk".
  const rand = noise(11);
  const out = buffer(sampleRate, 1.0);
  mix(out, slideWhistle(sampleRate, 2200, 260, 0.82, rand, 0.7), sampleRate, 0);
  mix(out, kick(sampleRate, rand, 0.18), sampleRate, 0.8, 1.3);
  mix(out, chime(sampleRate, hz(55), 0.18, rand, 14), sampleRate, 0.8, 0.5);
  return master(out, 2.2);
}

function trapdoor(sampleRate) {
  // Clunk, BOING, and a short whistle down the hole.
  const rand = noise(12);
  const out = buffer(sampleRate, 0.8);
  mix(out, kick(sampleRate, rand, 0.15), sampleRate, 0, 1.2);
  mix(out, snare(sampleRate, rand, 0.06), sampleRate, 0, 0.8);
  const boing = buffer(sampleRate, 0.5);
  let phase = 0;
  for (let i = 0; i < boing.length; i++) {
    const t = i / sampleRate;
    const f = 300 + 260 * Math.sin(TAU * 14 * t) * Math.exp(-t * 4);
    phase += (TAU * f) / sampleRate;
    boing[i] = (Math.sin(phase) + 0.5 * Math.sin(2 * phase)) * Math.exp(-t * 4);
  }
  mix(out, boing, sampleRate, 0.03);
  mix(out, slideWhistle(sampleRate, 1400, 500, 0.3, rand), sampleRate, 0.48, 0.6);
  return master(out, 2.5);
}

function gameOver(sampleRate) {
  // The sad trombone: WAH, WAH, WAH, WAAAAAH, with a tuba underneath.
  const out = buffer(sampleRate, 2.6);
  const notes = [
    [58, 0.42],
    [57, 0.42],
    [56, 0.42],
    [55, 1.3],
  ];
  let at = 0;
  for (const [note, seconds] of notes) {
    const long = seconds > 1;
    const tone = buffer(sampleRate, seconds);
    let phase = 0;
    let y = 0;
    for (let i = 0; i < tone.length; i++) {
      const t = i / sampleRate;
      const wobble = long ? 0.045 * Math.sin(TAU * 6 * t) * Math.min(1, t / 0.25) : 0;
      phase += (TAU * hz(note) * (1 + wobble)) / sampleRate;
      // The plunger mute opens on each note: the "wah".
      const cutoff = 500 + 3200 * Math.min(1, t / 0.08) * Math.exp(-t * (long ? 0.8 : 2.5));
      y += (1 - Math.exp((-TAU * cutoff) / sampleRate)) * (saw(phase) - y);
      tone[i] = y * envelope(t, seconds, 0.015, 0.06);
    }
    mix(out, tone, sampleRate, at);
    mix(out, brass(sampleRate, hz(note - 12), seconds, { bright: 900, vibrato: long ? 0.02 : 0 }), sampleRate, at, 0.5);
    at += seconds;
  }
  return master(out, 2.2);
}

// ---------------------------------------------------------------------------

/** Every sound, by track number. `durationMs` matches `Sound::duration_ms()`. */
export const SOUNDS = {
  1: { name: "Pop bumper", durationMs: 150, render: popBumper },
  2: { name: "Kicker", durationMs: 150, render: kicker },
  3: { name: "Lane chime", durationMs: 400, render: lane },
  4: { name: "30 points", durationMs: 600, render: thirtyPoints },
  5: { name: "Bonus collect", durationMs: 2000, render: bonusCollect },
  6: { name: "Special lit", durationMs: 1500, render: special },
  7: { name: "Replay (knocker)", durationMs: 300, render: replay },
  8: { name: "Tilt", durationMs: 1200, render: tilt },
  9: { name: "Game start", durationMs: 3400, render: gameStart },
  10: { name: "Player added", durationMs: 500, render: playerAdded },
  11: { name: "Ball serve", durationMs: 700, render: ballServe },
  12: { name: "Drain", durationMs: 1000, render: drain },
  13: { name: "Trapdoor", durationMs: 800, render: trapdoor },
  14: { name: "Game over", durationMs: 2600, render: gameOver },
  21: { name: "Bonus advance 20", durationMs: 300, render: bonusAdvance(1) },
  22: { name: "Bonus advance 30", durationMs: 300, render: bonusAdvance(2) },
  23: { name: "Bonus advance 40", durationMs: 300, render: bonusAdvance(3) },
  24: { name: "Bonus advance 50", durationMs: 300, render: bonusAdvance(4) },
  25: { name: "Bonus advance 60", durationMs: 300, render: bonusAdvance(5) },
  26: { name: "Bonus advance 70", durationMs: 300, render: bonusAdvance(6) },
  27: { name: "Bonus advance 80", durationMs: 300, render: bonusAdvance(7) },
  28: { name: "Bonus advance 90", durationMs: 300, render: bonusAdvance(8) },
  29: { name: "Bonus advance 100", durationMs: 300, render: bonusAdvance(9) },
};

/** Renders track `track` at `sampleRate`, already mastered. */
export function render(track, sampleRate) {
  return SOUNDS[track].render(sampleRate);
}
