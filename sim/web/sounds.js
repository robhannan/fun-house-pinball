// Sound effects for the Fun House: mostly what a 1950s machine sounded like
// (bells, relays, the score motor, the knocker), with circus extras (drum roll,
// fanfare, slide whistles, sad trombone). Coils make their own noise on the
// real machine, so kickers, flippers and pop-bumper solenoids have no sound
// here; the simulator plays recordings of them (COIL_SAMPLES) when coils fire.
//
// Each sound has a recorded version (`sample`, from free CC0 recordings: see
// docs/sound-credits.md) and this synthesised one as a fallback. Used in two
// places:
// - the simulator (sim.js) plays them with the Web Audio API
// - sim/render-sounds.mjs writes them to the DFPlayer Mini's SD card
//   (`pnpm run render:sounds`)
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

/** A pop bumper scoring: a short chime (the solenoid makes its own noise). */
const popBumper = (sampleRate) => master(chime(sampleRate, hz(76), 0.6, noise(1), 7), 2.2);

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
  // The tilt relay dropping out: a hard clunk and a click.
  const rand = noise(13);
  const out = buffer(sampleRate, 0.6);
  mix(out, kick(sampleRate, rand, 0.25), sampleRate, 0, 1);
  mix(out, snare(sampleRate, rand, 0.05), sampleRate, 0.01, 0.6);
  mix(out, snare(sampleRate, rand, 0.04), sampleRate, 0.22, 0.4);
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
  // A relay click, then a bell.
  const rand = noise(9);
  const out = buffer(sampleRate, 0.7);
  mix(out, snare(sampleRate, rand, 0.05), sampleRate, 0, 0.7);
  mix(out, chime(sampleRate, hz(79), 0.6, rand, 6), sampleRate, 0.1);
  return master(out, 2.2);
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
  // The ball dropping into the hole: a thunk and a rattle.
  const rand = noise(12);
  const out = buffer(sampleRate, 0.75);
  mix(out, kick(sampleRate, rand, 0.3), sampleRate, 0, 1.2);
  for (let t = 0.15; t < 0.6; t += 0.07) mix(out, snare(sampleRate, rand, 0.04), sampleRate, t, 0.4 * (1 - t));
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

/**
 * Every sound, by track number. `durationMs` matches `Sound::duration_ms()`.
 * `sample` is the recorded version (built by `pnpm run import:samples` from
 * free recordings; see docs/sound-credits.md); `render` is the synthesised one,
 * kept as a fallback and for comparison.
 */
export const SOUNDS = {
  1: { name: "Pop bumper", durationMs: 600, sample: "samples/001.wav", render: popBumper },
  3: { name: "Lane chime", durationMs: 800, sample: "samples/003.wav", render: lane },
  4: { name: "30 points", durationMs: 1100, sample: "samples/004.wav", render: thirtyPoints },
  5: { name: "Bonus collect", durationMs: 2400, sample: "samples/005.wav", render: bonusCollect },
  6: { name: "Special lit", durationMs: 2000, sample: "samples/006.wav", render: special },
  7: { name: "Replay (knocker)", durationMs: 400, sample: "samples/007.wav", render: replay },
  8: { name: "Tilt", durationMs: 600, sample: "samples/008.wav", render: tilt },
  9: { name: "Game start", durationMs: 2600, sample: "samples/009.wav", render: gameStart },
  10: { name: "Player added", durationMs: 700, sample: "samples/010.wav", render: playerAdded },
  11: { name: "Ball serve", durationMs: 800, sample: "samples/011.wav", render: ballServe },
  12: { name: "Drain", durationMs: 1450, sample: "samples/012.wav", render: drain },
  13: { name: "Trapdoor", durationMs: 750, sample: "samples/013.wav", render: trapdoor },
  14: { name: "Game over", durationMs: 4300, sample: "samples/014.wav", render: gameOver },
  21: { name: "Bonus advance 20", durationMs: 550, sample: "samples/021.wav", render: bonusAdvance(1) },
  22: { name: "Bonus advance 30", durationMs: 550, sample: "samples/022.wav", render: bonusAdvance(2) },
  23: { name: "Bonus advance 40", durationMs: 550, sample: "samples/023.wav", render: bonusAdvance(3) },
  24: { name: "Bonus advance 50", durationMs: 550, sample: "samples/024.wav", render: bonusAdvance(4) },
  25: { name: "Bonus advance 60", durationMs: 550, sample: "samples/025.wav", render: bonusAdvance(5) },
  26: { name: "Bonus advance 70", durationMs: 550, sample: "samples/026.wav", render: bonusAdvance(6) },
  27: { name: "Bonus advance 80", durationMs: 550, sample: "samples/027.wav", render: bonusAdvance(7) },
  28: { name: "Bonus advance 90", durationMs: 550, sample: "samples/028.wav", render: bonusAdvance(8) },
  29: { name: "Bonus advance 100", durationMs: 550, sample: "samples/029.wav", render: bonusAdvance(9) },
};

/**
 * Renders track `track`'s synthesised version at `sampleRate`, already
 * mastered, and trimmed (with a short fade) to its `durationMs`.
 */
export function render(track, sampleRate) {
  const { render: synthesise, durationMs } = SOUNDS[track];
  const samples = synthesise(sampleRate);
  const length = Math.min(samples.length, Math.floor((durationMs / 1000) * sampleRate));
  const fade = Math.min(length, Math.round(0.03 * sampleRate));
  for (let i = 0; i < fade; i++) samples[length - 1 - i] *= i / fade;
  return samples.subarray(0, length);
}

/** Simulator only: the noises the real coils make, by coil number. */
export const COIL_SAMPLES = {
  1: "samples/coil-kicker.wav",
  2: "samples/coil-kicker.wav",
  3: "samples/coil-pop.wav",
  4: "samples/coil-pop.wav",
  5: "samples/coil-pop.wav",
  6: "samples/coil-pop.wav",
  7: "samples/coil-pop.wav",
  8: "samples/coil-flipper.wav",
  10: "samples/coil-flipper.wav",
};

/** Recorded real bells for the optional bell coils (1s, 10s, 100s). */
export const BELL_SAMPLES = ["samples/bell-1.wav", "samples/bell-10.wav", "samples/bell-100.wav"];
