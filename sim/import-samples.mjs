// Builds the recorded sounds in sim/web/samples/ from free recordings (CC0 on
// Freesound, or public domain on Wikimedia Commons), as listed in
// sim/samples/sources.json, and writes docs/sound-credits.md. Run with:
//   pnpm run import:samples
//
// Needs ffmpeg. Downloads each recording once into target/sample-cache/. For
// each sound it cuts the listed segments ("layers"), pitch-shifts or speeds
// them up if asked, mixes them, fades the ends, and masters the result loud
// (soft-clipped to a 0.97 peak), like the synthesised sounds.
//
// A sound with a "music" entry is the attract-mode music: its beats are found
// in the recording, it's cut at the end of a phrase, and the beat times are
// written to crates/pinball-core/src/music_beats.rs so the lamps can dance in
// time. Its length is written to sim/web/sounds.js too.
import { execFileSync } from "node:child_process";

import { fs, path } from "zx";

import { SAMPLE_RATE, encodeWav } from "./wav.mjs";

const root = path.resolve(import.meta.dirname, "..");
const config = await fs.readJson(path.join(root, "sim/samples/sources.json"));
const cache = path.join(root, "target/sample-cache");
const out = path.join(root, "sim/web/samples");
await fs.ensureDir(cache);
await fs.ensureDir(out);

/** The cached recording for a source id, downloading it if needed. */
async function recording(id) {
  const source = config.sources[id];
  if (!source) throw new Error(`source ${id} isn't in "sources"`);
  const url = source.download ?? source.preview;
  const extension = path.extname(new URL(url).pathname) || ".mp3";
  const file = path.join(cache, `${id}${extension}`);
  if (!(await fs.pathExists(file))) {
    const response = await fetch(url, { headers: { "User-Agent": "fun-house-pinball (sound import)" } });
    if (!response.ok) throw new Error(`downloading ${id}: HTTP ${response.status}`);
    await fs.writeFile(file, Buffer.from(await response.arrayBuffer()));
  }
  return file;
}

const sourceOf = (l) => String(l.source ?? l.freesound);

/** One layer as mono float samples at SAMPLE_RATE. */
async function layer(l) {
  const { start, length, semitones = 0, speed = 1 } = l;
  const filters = [];
  if (semitones !== 0) {
    // Resampling shifts pitch and speed together, as a bigger or smaller bell would.
    filters.push(`asetrate=${Math.round(SAMPLE_RATE * 2 ** (semitones / 12))}`, `aresample=${SAMPLE_RATE}`);
  }
  for (let s = speed; s > 1.0001; s /= Math.min(s, 2)) filters.push(`atempo=${Math.min(s, 2)}`);
  const args = ["-v", "error", "-ss", String(start), "-t", String(length), "-i", await recording(sourceOf(l)), "-ac", "1", "-ar", String(SAMPLE_RATE)];
  if (filters.length > 0) args.push("-af", filters.join(","));
  // execFileSync rather than zx's $, which decodes output as text and would
  // corrupt the raw audio.
  const bytes = execFileSync("ffmpeg", [...args, "-f", "f32le", "-"], { maxBuffer: 1 << 30 });
  return new Float32Array(bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.length));
}

/** Mixes the layers. */
async function mixLayers(layers) {
  const parts = await Promise.all(layers.map(async (l) => ({ samples: await layer(l), at: l.at ?? 0, gain: l.gain ?? 1 })));
  const length = Math.max(...parts.map((p) => Math.round(p.at * SAMPLE_RATE) + p.samples.length));
  const mix = new Float32Array(length);
  for (const { samples, at, gain } of parts) {
    const offset = Math.round(at * SAMPLE_RATE);
    for (let i = 0; i < samples.length; i++) mix[offset + i] += samples[i] * gain;
  }
  return mix;
}

function fade(samples, fadeIn, fadeOut) {
  const fadeInLength = Math.round(fadeIn * SAMPLE_RATE);
  const fadeOutLength = Math.round(fadeOut * SAMPLE_RATE);
  for (let i = 0; i < fadeInLength && i < samples.length; i++) samples[i] *= i / fadeInLength;
  for (let i = 0; i < fadeOutLength && i < samples.length; i++) samples[samples.length - 1 - i] *= i / fadeOutLength;
  return samples;
}

/** Soft-clips with tanh and scales to `level` times a 0.97 peak. */
function master(samples, drive, level = 1) {
  let peak = 0;
  for (const s of samples) peak = Math.max(peak, Math.abs(s));
  if (peak === 0) throw new Error("silent");
  let outPeak = 0;
  for (let i = 0; i < samples.length; i++) {
    samples[i] = Math.tanh((drive * samples[i]) / peak);
    outPeak = Math.max(outPeak, Math.abs(samples[i]));
  }
  for (let i = 0; i < samples.length; i++) samples[i] *= (0.97 * level) / outPeak;
  return samples;
}

/**
 * Finds the beats in `samples`, in ms. The onset strength is the rise in
 * energy (low and high bands) every 10 ms; the tempo is the repeat that best
 * matches it (70–200 BPM); then each beat is predicted from the last and moved
 * to the strongest onset nearby, so a live band's drifting tempo is followed.
 */
function findBeats(samples) {
  const hop = Math.round(SAMPLE_RATE * 0.01);
  const frames = Math.floor(samples.length / hop);
  const low = new Float64Array(frames);
  const high = new Float64Array(frames);
  let y = 0;
  for (let k = 0; k < frames; k++) {
    let el = 0;
    let eh = 0;
    for (let i = k * hop; i < (k + 1) * hop; i++) {
      y += 0.025 * (samples[i] - y);
      el += y * y;
      eh += (samples[i] - y) ** 2;
    }
    low[k] = Math.log(el + 1e-9);
    high[k] = Math.log(eh + 1e-9);
  }
  const onset = new Float64Array(frames);
  for (let k = 1; k < frames; k++) onset[k] = Math.max(0, low[k] - low[k - 1]) + Math.max(0, high[k] - high[k - 1]);

  // Autocorrelation of the onsets, at lags of 0.15–1.8 s.
  const autocorrelation = (l) => {
    let r = 0;
    for (let k = l; k < frames; k++) r += onset[k] * onset[k - l];
    return r / (frames - l);
  };
  // Score each tempo by its own lag plus double its lag (a real beat repeats
  // every other beat too), weighted towards ~120 BPM, where most music sits.
  // Without that, a pattern every third beat can win (e.g. 81 instead of 122).
  let lag = 0;
  let best = -1;
  for (let l = 30; l <= 86; l++) {
    const bpm = 6000 / l;
    const prior = Math.exp(-0.5 * (Math.log2(bpm / 120) / 0.5) ** 2);
    const score = (autocorrelation(l) + 0.5 * autocorrelation(2 * l)) * prior;
    if (score > best) {
      best = score;
      lag = l;
    }
  }
  let phase = 0;
  best = -1;
  for (let p = 0; p < lag; p++) {
    let sum = 0;
    for (let k = p; k < frames; k += lag) sum += onset[k];
    if (sum > best) {
      best = sum;
      phase = p;
    }
  }

  const window = 6; // ±60 ms
  const beats = [];
  let period = lag;
  for (let predicted = phase; predicted < frames; ) {
    const centre = Math.round(predicted);
    // Never snap back to (or before) the last beat.
    const earliest = beats.length > 0 ? beats.at(-1) + Math.ceil(lag / 2) : 0;
    let at = Math.max(centre, earliest);
    for (let k = Math.max(earliest, centre - window); k <= Math.min(frames - 1, centre + window); k++) if (onset[k] > onset[at]) at = k;
    if (beats.length > 0) {
      // Follow tempo drift slowly, within 20% of the detected tempo.
      period = Math.min(1.2 * lag, Math.max(0.8 * lag, 0.85 * period + 0.15 * (at - beats.at(-1))));
    }
    beats.push(at);
    predicted = at + period;
  }
  return { bpm: 6000 / lag, beats: beats.map((k) => k * 10), onset };
}

/**
 * Cuts a seamless loop from the music: from the first beat to a later beat a
 * whole number of phrases on. Of the candidate lengths in `music.loopBeats`, it
 * picks the one whose next few beats have the most similar harmony to the
 * first few (comparing their chroma), so the jump back to the start fits the
 * music. Over the last `music.crossfade` seconds the loop fades into the audio
 * just before its start, which leads straight into its first sample, so the
 * seam is smooth.
 */
function loopMusic(samples, music) {
  const { bpm, beats } = findBeats(samples);
  // Harmony after a point: the chroma (energy in each of the 12 pitch classes)
  // of each of the next 4 beats, joined into one vector.
  const harmonyAfter = (beat) => {
    const vector = [];
    for (let b = beat; b < beat + 4; b++) vector.push(...chroma(samples, beats[b], beats[b + 1]));
    return vector;
  };
  const similarity = (x, y) => {
    let xy = 0;
    let xx = 0;
    let yy = 0;
    for (let i = 0; i < x.length; i++) {
      xy += x[i] * y[i];
      xx += x[i] * x[i];
      yy += y[i] * y[i];
    }
    return xy / Math.sqrt(xx * yy);
  };
  const atStart = harmonyAfter(0);
  const candidates = music.loopBeats.filter((n) => n + 5 < beats.length);
  if (candidates.length === 0) throw new Error("the recording is too short for any loop length");
  let loopBeats = candidates[0];
  let best = -Infinity;
  for (const n of candidates) {
    const score = similarity(atStart, harmonyAfter(n));
    console.log(`  loop of ${n} beats: similarity ${score.toFixed(2)}`);
    if (score > best) {
      best = score;
      loopBeats = n;
    }
  }
  const start = Math.round((beats[0] / 1000) * SAMPLE_RATE);
  const end = Math.round((beats[loopBeats] / 1000) * SAMPLE_RATE);
  const loop = samples.slice(start, end);
  const blend = Math.round(music.crossfade * SAMPLE_RATE);
  for (let i = 0; i < blend; i++) {
    const t = i / blend;
    // Fade from the audio before the loop's end to the audio before its start,
    // which leads straight into the first sample of the loop.
    loop[loop.length - blend + i] = samples[end - blend + i] * (1 - t) + samples[start - blend + i] * t;
  }
  const times = beats.slice(0, loopBeats).map((ms) => ms - beats[0]);
  return { bpm, beats: { loop, times }, loopBeats };
}

/**
 * The chroma of `samples` between `fromMs` and `toMs`: how much energy falls in
 * each of the 12 pitch classes (C, C#, ... B), from 2048-point FFTs between
 * 80 Hz and 5 kHz, normalised to sum to 1.
 */
function chroma(samples, fromMs, toMs) {
  const size = 2048;
  const classes = new Array(12).fill(0);
  const from = Math.round((fromMs / 1000) * SAMPLE_RATE);
  const to = Math.round((toMs / 1000) * SAMPLE_RATE);
  for (let at = from; at + size <= to; at += size / 2) {
    const re = new Float64Array(size);
    const im = new Float64Array(size);
    for (let i = 0; i < size; i++) re[i] = samples[at + i] * (0.5 - 0.5 * Math.cos((2 * Math.PI * i) / size));
    fft(re, im);
    for (let k = 1; k < size / 2; k++) {
      const freq = (k * SAMPLE_RATE) / size;
      if (freq < 80 || freq > 5_000) continue;
      const note = Math.round(12 * Math.log2(freq / 440) + 69);
      classes[((note % 12) + 12) % 12] += Math.hypot(re[k], im[k]);
    }
  }
  const total = classes.reduce((a, b) => a + b, 0) || 1;
  return classes.map((c) => c / total);
}

/** In-place radix-2 FFT. */
function fft(re, im) {
  const n = re.length;
  for (let i = 1, j = 0; i < n; i++) {
    let bit = n >> 1;
    for (; j & bit; bit >>= 1) j ^= bit;
    j ^= bit;
    if (i < j) {
      [re[i], re[j]] = [re[j], re[i]];
      [im[i], im[j]] = [im[j], im[i]];
    }
  }
  for (let len = 2; len <= n; len <<= 1) {
    const angle = (-2 * Math.PI) / len;
    for (let i = 0; i < n; i += len) {
      for (let j = 0; j < len / 2; j++) {
        const wr = Math.cos(angle * j);
        const wi = Math.sin(angle * j);
        const ur = re[i + j];
        const ui = im[i + j];
        const vr = re[i + j + len / 2] * wr - im[i + j + len / 2] * wi;
        const vi = re[i + j + len / 2] * wi + im[i + j + len / 2] * wr;
        re[i + j] = ur + vr;
        im[i + j] = ui + vi;
        re[i + j + len / 2] = ur - vr;
        im[i + j + len / 2] = ui - vi;
      }
    }
  }
}

/** Writes the music's beats and length for the core. */
async function writeBeats(name, beats, lengthMs, bpm) {
  const rows = [];
  for (let i = 0; i < beats.length; i += 12) rows.push(`    ${beats.slice(i, i + 12).join(", ")},`);
  await fs.writeFile(
    path.join(root, "crates/pinball-core/src/music_beats.rs"),
    `//! Beat times for the attract-mode music ("${name}"), in milliseconds from
//! its start, found in the recording at about ${bpm.toFixed(0)} beats per minute.
//!
//! Generated by \`pnpm run import:samples\` from \`sim/samples/sources.json\`;
//! don't edit by hand.

/// How long the music lasts, in milliseconds.
pub const ATTRACT_MUSIC_MS: u32 = ${lengthMs};

/// When each beat falls, in milliseconds from the start of the music.
pub const ATTRACT_MUSIC_BEATS_MS: [u32; ${beats.length}] = [
${rows.join("\n")}
];
`,
  );
}

/** Sets a track's \`durationMs\` in sim/web/sounds.js. */
async function setDuration(track, ms) {
  const file = path.join(root, "sim/web/sounds.js");
  const text = await fs.readFile(file, "utf8");
  const pattern = new RegExp(`(\\n  ${track}: \\{ name: "[^"]+", durationMs: )\\d+`);
  if (!pattern.test(text)) throw new Error(`track ${track} isn't in sounds.js`);
  await fs.writeFile(file, text.replace(pattern, `$1${ms}`));
}

const credits = new Map();
const entries = [
  ...Object.entries(config.tracks).map(([track, spec]) => [track, track.padStart(3, "0"), spec]),
  ...Object.entries(config.extras).map(([name, spec]) => [undefined, name, spec]),
];
for (const [track, file, spec] of entries) {
  let samples = await mixLayers(spec.layers);
  if (spec.music) {
    const { bpm, beats, loopBeats } = loopMusic(samples, spec.music);
    samples = master(beats.loop, spec.drive ?? 2.5, spec.level);
    const lengthMs = Math.floor((samples.length / SAMPLE_RATE) * 1000);
    await writeBeats(spec.name, beats.times, lengthMs, bpm);
    await setDuration(track, lengthMs);
    console.log(`  a ${loopBeats}-beat loop at about ${bpm.toFixed(0)} BPM -> crates/pinball-core/src/music_beats.rs`);
  } else {
    fade(samples, spec.fadeIn ?? 0.005, spec.fadeOut ?? 0.05);
    master(samples, spec.drive ?? 2.5, spec.level);
  }
  await fs.writeFile(path.join(out, `${file}.wav`), encodeWav(samples));
  const ms = Math.round((samples.length / SAMPLE_RATE) * 1000);
  console.log(`sim/web/samples/${file}.wav  ${spec.name} (${ms} ms)`);
  for (const l of spec.layers) {
    if (!credits.has(sourceOf(l))) credits.set(sourceOf(l), new Set());
    credits.get(sourceOf(l)).add(spec.name);
  }
}

const rows = [...credits].map(([id, uses]) => {
  const s = config.sources[id];
  return `| [${s.title}](${s.page}) | ${s.author} | ${s.licence} | ${[...uses].join(", ")} |`;
});
await fs.writeFile(
  path.join(root, "docs/sound-credits.md"),
  `# Sound credits

The recorded sounds in \`sim/web/samples/\` are cut from these recordings. The
effects come from [Freesound](https://freesound.org) and are
[CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/); the attract-mode
music comes from [Wikimedia Commons](https://commons.wikimedia.org) and is in
the public domain. No credit is required, but thank you to everyone who shared
them.

This file is generated from \`sim/samples/sources.json\` by
\`pnpm run import:samples\`; edit that file, not this one.

| Recording | By | Licence | Used for |
|---|---|---|---|
${rows.join("\n")}
`,
);
console.log(`docs/sound-credits.md  (${rows.length} recordings)`);
