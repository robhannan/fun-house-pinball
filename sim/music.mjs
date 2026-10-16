// Music analysis for the attract-mode loop: finding beats, and cutting a
// seamless loop at a phrase boundary. Used by sim/import-samples.mjs.

import { SAMPLE_RATE } from "./wav.mjs";

/**
 * Finds the beats in `samples`, in ms. The onset strength is the rise in
 * energy (low and high bands) every 10 ms; the tempo is the repeat that best
 * matches it (70–200 BPM); then each beat is predicted from the last and moved
 * to the strongest onset nearby, so a live band's drifting tempo is followed.
 */
export function findBeats(samples) {
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
export function loopMusic(samples, music) {
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
  // The seam blends in the audio just before the loop's first beat, so start
  // on the first beat with at least that much audio before it.
  const first = beats.findIndex((ms) => ms >= music.crossfade * 1000);
  const atStart = harmonyAfter(first);
  const candidates = music.loopBeats.filter((n) => first + n + 5 < beats.length);
  if (candidates.length === 0) throw new Error("the recording is too short for any loop length");
  let loopBeats = candidates[0];
  let best = -Infinity;
  for (const n of candidates) {
    const score = similarity(atStart, harmonyAfter(first + n));
    console.log(`  loop of ${n} beats: similarity ${score.toFixed(2)}`);
    if (score > best) {
      best = score;
      loopBeats = n;
    }
  }
  const start = Math.round((beats[first] / 1000) * SAMPLE_RATE);
  const end = Math.round((beats[first + loopBeats] / 1000) * SAMPLE_RATE);
  const loop = samples.slice(start, end);
  const blend = Math.round(music.crossfade * SAMPLE_RATE);
  for (let i = 0; i < blend; i++) {
    const t = i / blend;
    // Fade from the audio before the loop's end to the audio before its start,
    // which leads straight into the first sample of the loop.
    loop[loop.length - blend + i] = samples[end - blend + i] * (1 - t) + samples[start - blend + i] * t;
  }
  if (loop.some(Number.isNaN)) throw new Error("the loop has invalid samples");
  const times = beats.slice(first, first + loopBeats).map((ms) => ms - beats[first]);
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
