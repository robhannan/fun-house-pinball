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
// A sound with a "music" entry is the attract-mode music. Its beats are found
// in the recording and written, with its length, to
// crates/pinball-core/src/music_beats.rs so the lamps can dance in time; the
// length goes into sim/web/sounds.js too. With "full", the whole piece is used
// (it repeats from the top); otherwise a seamless loop is cut at a phrase
// boundary (sim/music.mjs). "format": "mp3" stores a long sound compressed.
import { execFileSync } from "node:child_process";

import { fs, path } from "zx";

import { findBeats, loopMusic } from "./music.mjs";
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

/** A Rust integer literal with digit separators, as clippy wants: 155_220. */
const rustInt = (n) => String(n).replace(/\B(?=(\d{3})+$)/g, "_");

/** Writes the music's beats and length for the core. */
async function writeBeats(name, beats, lengthMs, bpm) {
  const rows = [];
  for (let i = 0; i < beats.length; i += 12) rows.push(`    ${beats.slice(i, i + 12).map(rustInt).join(", ")},`);
  await fs.writeFile(
    path.join(root, "crates/pinball-core/src/music_beats.rs"),
    `//! Beat times for the attract-mode music ("${name}"), in milliseconds from
//! its start, found in the recording at about ${bpm.toFixed(0)} beats per minute.
//!
//! Generated by \`pnpm run import:samples\` from \`sim/samples/sources.json\`;
//! don't edit by hand.

/// How long the music lasts, in milliseconds.
pub const ATTRACT_MUSIC_MS: u32 = ${rustInt(lengthMs)};

/// When each beat falls, in milliseconds from the start of the music.
pub const ATTRACT_MUSIC_BEATS_MS: [u32; ${beats.length}] = [
${rows.join("\n")}
];
`,
  );
  // Format it as cargo fmt would, so a fresh import matches the committed file.
  execFileSync("rustfmt", ["--edition", "2024", path.join(root, "crates/pinball-core/src/music_beats.rs")]);
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
  if (spec.music?.full) {
    // The whole piece, played through and then repeated.
    fade(samples, spec.fadeIn ?? 0.005, spec.fadeOut ?? 0.3);
    master(samples, spec.drive ?? 2.5, spec.level);
    const lengthMs = Math.floor((samples.length / SAMPLE_RATE) * 1000);
    const { bpm, beats } = findBeats(samples);
    await writeBeats(spec.name, beats.filter((ms) => ms < lengthMs), lengthMs, bpm);
    await setDuration(track, lengthMs);
    console.log(`  the full piece, ${beats.length} beats at about ${bpm.toFixed(0)} BPM -> crates/pinball-core/src/music_beats.rs`);
  } else if (spec.music) {
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
  if (spec.format === "mp3") {
    // Long sounds (the music) are stored as MP3 to keep the repo small.
    const wav = path.join(cache, `${file}.wav`);
    await fs.writeFile(wav, encodeWav(samples));
    execFileSync("ffmpeg", ["-v", "error", "-y", "-i", wav, "-codec:a", "libmp3lame", "-b:a", "160k", path.join(out, `${file}.mp3`)]);
    await fs.remove(path.join(out, `${file}.wav`));
  } else {
    await fs.writeFile(path.join(out, `${file}.wav`), encodeWav(samples));
  }
  const ms = Math.round((samples.length / SAMPLE_RATE) * 1000);
  console.log(`sim/web/samples/${file}.${spec.format ?? "wav"}  ${spec.name} (${ms} ms)`);
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
