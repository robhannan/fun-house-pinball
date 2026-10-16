// Sound effects for the Fun House, synthesised from scratch so there are no
// recordings to license. A mix of classic pinball (bells, thumps, the knocker,
// the tilt buzzer) and circus (calliope, drum roll, clown horn, slide whistles,
// sad trombone).
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

/** Scales to a peak of 0.8. */
function normalise(samples) {
  let peak = 0;
  for (const s of samples) peak = Math.max(peak, Math.abs(s));
  if (peak > 0) for (let i = 0; i < samples.length; i++) samples[i] *= 0.8 / peak;
  return samples;
}

// ---------------------------------------------------------------------------
// Instruments

/** A struck bell: inharmonic partials that die away, higher ones faster. */
function bell(sampleRate, freq, seconds, decay = 7) {
  const out = buffer(sampleRate, seconds);
  const partials = [
    [1, 1],
    [2.76, 0.45],
    [5.4, 0.25],
    [8.93, 0.12],
  ];
  for (let i = 0; i < out.length; i++) {
    const t = i / sampleRate;
    let s = 0;
    for (const [ratio, gain] of partials) {
      s += gain * Math.sin(TAU * freq * ratio * t) * Math.exp(-t * decay * ratio ** 0.7);
    }
    out[i] = s * Math.min(1, t / 0.002);
  }
  return out;
}

/** A calliope pipe: breathy, with a steam-organ wobble. */
function pipe(sampleRate, freq, seconds, rand) {
  const out = buffer(sampleRate, seconds);
  let phase = 0;
  for (let i = 0; i < out.length; i++) {
    const t = i / sampleRate;
    const f = freq * (1 + 0.012 * Math.sin(TAU * 6 * t));
    phase += (TAU * f) / sampleRate;
    const tone = Math.sin(phase) + 0.5 * Math.sin(2 * phase) + 0.3 * Math.sin(3 * phase) + 0.15 * Math.sin(4 * phase);
    out[i] = (tone + 0.08 * rand()) * envelope(t, seconds, 0.015, 0.03);
  }
  return out;
}

/** Plays `[midiNote, beats]` pairs on the calliope. */
function calliope(sampleRate, notes, beat, rand, octaveBass = false) {
  const total = notes.reduce((sum, [, beats]) => sum + beats, 0) * beat;
  const out = buffer(sampleRate, total + 0.05);
  let at = 0;
  for (const [note, beats] of notes) {
    if (note !== null) mix(out, pipe(sampleRate, hz(note), beats * beat * 0.92, rand), sampleRate, at);
    at += beats * beat;
  }
  if (octaveBass) {
    // Oom-pah: a low note on each beat, root and fifth.
    for (let b = 0, n = 0; b < total - beat / 2; b += beat * 2, n++) {
      mix(out, pipe(sampleRate, hz(n % 2 ? 43 : 36), beat * 0.6, rand), sampleRate, b, 0.6);
    }
  }
  return out;
}

/** A slide whistle gliding from `from` to `to` Hz. */
function slideWhistle(sampleRate, from, to, seconds, rand) {
  const out = buffer(sampleRate, seconds);
  let phase = 0;
  for (let i = 0; i < out.length; i++) {
    const t = i / sampleRate;
    const glide = from * (to / from) ** Math.min(1, t / (seconds - 0.08));
    const f = glide * (1 + 0.03 * Math.sin(TAU * 7 * t));
    phase += (TAU * f) / sampleRate;
    out[i] = (Math.sin(phase) + 0.04 * rand()) * envelope(t, seconds, 0.03, 0.08);
  }
  return out;
}

/** A drum hit: noise (snare) or a falling sine (bass). */
function drum(sampleRate, seconds, rand, { bass = false, decay = 30 } = {}) {
  const out = buffer(sampleRate, seconds);
  let phase = 0;
  for (let i = 0; i < out.length; i++) {
    const t = i / sampleRate;
    if (bass) {
      phase += (TAU * (50 + 90 * Math.exp(-t * 30))) / sampleRate;
      out[i] = Math.sin(phase) * Math.exp(-t * decay);
    } else {
      out[i] = rand() * Math.exp(-t * decay);
    }
  }
  return out;
}

// ---------------------------------------------------------------------------
// The sounds

function popBumper(sampleRate) {
  const rand = noise(1);
  const out = buffer(sampleRate, 0.14);
  let phase = 0;
  for (let i = 0; i < out.length; i++) {
    const t = i / sampleRate;
    phase += (TAU * (55 + 110 * Math.exp(-t * 20))) / sampleRate;
    out[i] = Math.sin(phase) * Math.exp(-t * 22) + 0.4 * rand() * Math.exp(-t * 120);
  }
  return out;
}

function kicker(sampleRate) {
  const rand = noise(2);
  const smooth = lowpass(sampleRate, 2500);
  const out = buffer(sampleRate, 0.14);
  for (let i = 0; i < out.length; i++) {
    const t = i / sampleRate;
    out[i] = smooth(rand()) * Math.exp(-t * 45) + 0.6 * Math.sin(TAU * 230 * t) * Math.exp(-t * 35);
  }
  return out;
}

const lane = (sampleRate) => bell(sampleRate, hz(88), 0.4, 6);

function thirtyPoints(sampleRate) {
  const out = buffer(sampleRate, 0.6);
  [88, 91, 96].forEach((note, n) => mix(out, bell(sampleRate, hz(note), 0.36, 7), sampleRate, n * 0.12));
  return out;
}

/** The bonus wheel's bell climbs a major scale as the wheel goes round. */
function bonusAdvance(step) {
  const scale = [0, 2, 4, 5, 7, 9, 11, 12, 14, 16];
  return (sampleRate) => bell(sampleRate, hz(72 + scale[step]), 0.3, 9);
}

function bonusCollect(sampleRate) {
  const rand = noise(3);
  const out = buffer(sampleRate, 1.6);
  // A snare roll that builds, then a bass drum and cymbal crash.
  for (let t = 0; t < 1.05; t += 0.032) {
    mix(out, drum(sampleRate, 0.06, rand, { decay: 60 }), sampleRate, t, 0.25 + 0.6 * (t / 1.05));
  }
  mix(out, drum(sampleRate, 0.4, rand, { bass: true, decay: 9 }), sampleRate, 1.08, 1.2);
  const crash = buffer(sampleRate, 0.52);
  const dull = lowpass(sampleRate, 5000);
  for (let i = 0; i < crash.length; i++) {
    const x = rand();
    crash[i] = (x - dull(x)) * Math.exp((-i / sampleRate) * 5);
  }
  return mix(out, crash, sampleRate, 1.08, 1.4);
}

function special(sampleRate) {
  // A "ta-da" fanfare on the calliope.
  const notes = [
    [72, 1],
    [76, 1],
    [79, 1],
    [84, 4],
  ];
  return calliope(sampleRate, notes, 0.16, noise(4));
}

function replay(sampleRate) {
  // The knocker: a hard hit on the cabinet.
  const rand = noise(5);
  const out = buffer(sampleRate, 0.28);
  let phase = 0;
  for (let i = 0; i < out.length; i++) {
    const t = i / sampleRate;
    phase += (TAU * (60 + 140 * Math.exp(-t * 60))) / sampleRate;
    out[i] = Math.sin(phase) * Math.exp(-t * 18) + 0.5 * Math.sin(TAU * 470 * t) * Math.exp(-t * 60) + 0.6 * rand() * Math.exp(-t * 300);
  }
  return out;
}

function tilt(sampleRate) {
  // The tilt buzzer: a harsh square wave, chopped.
  const out = buffer(sampleRate, 1.2);
  const smooth = lowpass(sampleRate, 3000);
  for (let i = 0; i < out.length; i++) {
    const t = i / sampleRate;
    const square = Math.sign(Math.sin(TAU * 110 * t));
    const chop = Math.sin(TAU * 28 * t) > -0.3 ? 1 : 0.2;
    out[i] = smooth(square * chop) * envelope(t, 1.2, 0.01, 0.3);
  }
  return out;
}

function gameStart(sampleRate) {
  // An original circus march on the calliope, with an oom-pah bass.
  const melody = [
    [67, 1],
    [68, 1],
    [69, 1],
    [72, 1],
    [76, 2],
    [74, 1],
    [72, 1],
    [71, 1],
    [72, 1],
    [74, 1],
    [76, 1],
    [79, 3],
    [77, 1],
    [76, 1],
    [74, 1],
    [72, 3],
  ];
  return calliope(sampleRate, melody, 0.17, noise(6), true);
}

function playerAdded(sampleRate) {
  // A clown's bike horn: two honks.
  const rand = noise(7);
  const out = buffer(sampleRate, 0.5);
  const honk = (pitch) => {
    const h = buffer(sampleRate, 0.18);
    const nasal = lowpass(sampleRate, 1800);
    let phase = 0;
    for (let i = 0; i < h.length; i++) {
      const t = i / sampleRate;
      phase += (TAU * pitch * (1 - 0.06 * t)) / sampleRate;
      const saw = 2 * ((phase / TAU) % 1) - 1;
      h[i] = nasal(Math.tanh(3 * saw) + 0.1 * rand()) * envelope(t, 0.18, 0.01, 0.04);
    }
    return h;
  };
  mix(out, honk(380), sampleRate, 0);
  return mix(out, honk(340), sampleRate, 0.24);
}

const ballServe = (sampleRate) => slideWhistle(sampleRate, 420, 1600, 0.7, noise(8));
const drain = (sampleRate) => slideWhistle(sampleRate, 1600, 330, 0.9, noise(9));

function trapdoor(sampleRate) {
  // A cartoon spring: a wobbling pitch that settles and fades.
  const out = buffer(sampleRate, 0.7);
  let phase = 0;
  for (let i = 0; i < out.length; i++) {
    const t = i / sampleRate;
    const f = 260 + 180 * Math.sin(TAU * 13 * t) * Math.exp(-t * 4) - 80 * t;
    phase += (TAU * f) / sampleRate;
    out[i] = (Math.sin(phase) + 0.3 * Math.sin(2 * phase)) * Math.exp(-t * 4) * Math.min(1, t / 0.005);
  }
  return out;
}

function gameOver(sampleRate) {
  // The sad trombone: wah, wah, wah, waaah.
  const notes = [
    [58, 0.45],
    [57, 0.45],
    [56, 0.45],
    [55, 1.2],
  ];
  const out = buffer(sampleRate, 2.6);
  let at = 0;
  for (const [note, seconds] of notes) {
    const tone = buffer(sampleRate, seconds);
    let phase = 0;
    let y = 0;
    for (let i = 0; i < tone.length; i++) {
      const t = i / sampleRate;
      const wobble = seconds > 1 ? 0.03 * Math.sin(TAU * 5 * t) * Math.min(1, t / 0.3) : 0;
      phase += (TAU * hz(note) * (1 + wobble)) / sampleRate;
      const saw = 2 * ((phase / TAU) % 1) - 1;
      // The "wah": the mute opens at the start of each note.
      const cutoff = 400 + 1400 * Math.min(1, t / 0.12) * Math.exp(-t * 1.5);
      y += (1 - Math.exp((-TAU * cutoff) / sampleRate)) * (saw - y);
      tone[i] = y * envelope(t, seconds, 0.02, 0.08);
    }
    mix(out, tone, sampleRate, at);
    at += seconds;
  }
  return out;
}

// ---------------------------------------------------------------------------

/** Every sound, by track number. `durationMs` matches `Sound::duration_ms()`. */
export const SOUNDS = {
  1: { name: "Pop bumper", durationMs: 150, render: popBumper },
  2: { name: "Kicker", durationMs: 150, render: kicker },
  3: { name: "Lane bell", durationMs: 400, render: lane },
  4: { name: "30 points", durationMs: 600, render: thirtyPoints },
  5: { name: "Bonus collect", durationMs: 1600, render: bonusCollect },
  6: { name: "Special lit", durationMs: 1200, render: special },
  7: { name: "Replay (knocker)", durationMs: 300, render: replay },
  8: { name: "Tilt", durationMs: 1200, render: tilt },
  9: { name: "Game start", durationMs: 4000, render: gameStart },
  10: { name: "Player added", durationMs: 500, render: playerAdded },
  11: { name: "Ball serve", durationMs: 700, render: ballServe },
  12: { name: "Drain", durationMs: 900, render: drain },
  13: { name: "Trapdoor", durationMs: 700, render: trapdoor },
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

/** Renders track `track` at `sampleRate`, normalised. */
export function render(track, sampleRate) {
  return normalise(SOUNDS[track].render(sampleRate));
}
