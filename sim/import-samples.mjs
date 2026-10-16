// Builds the recorded sounds in sim/web/samples/ from free (CC0) recordings on
// Freesound, as listed in sim/samples/sources.json, and writes
// docs/sound-credits.md. Run with:
//   pnpm run import:samples
//
// Needs ffmpeg. Downloads each recording's preview once into
// target/sample-cache/. For each sound it cuts the listed segments ("layers"),
// pitch-shifts or speeds them up if asked, mixes them, fades the ends, and
// masters the result loud (soft-clipped to a 0.97 peak), like the synthesised
// sounds.
import { execFileSync } from "node:child_process";

import { fs, path } from "zx";

import { SAMPLE_RATE, encodeWav } from "./wav.mjs";

const root = path.resolve(import.meta.dirname, "..");
const config = await fs.readJson(path.join(root, "sim/samples/sources.json"));
const cache = path.join(root, "target/sample-cache");
const out = path.join(root, "sim/web/samples");
await fs.ensureDir(cache);
await fs.ensureDir(out);

/** The cached preview for a Freesound id, downloading it if needed. */
async function recording(id) {
  const file = path.join(cache, `${id}.mp3`);
  if (!(await fs.pathExists(file))) {
    const source = config.sources[id];
    if (!source) throw new Error(`freesound ${id} isn't in "sources"`);
    const response = await fetch(source.preview);
    if (!response.ok) throw new Error(`downloading ${id}: HTTP ${response.status}`);
    await fs.writeFile(file, Buffer.from(await response.arrayBuffer()));
  }
  return file;
}

/** One layer as mono float samples at SAMPLE_RATE. */
async function layer({ freesound, start, length, semitones = 0, speed = 1 }) {
  const filters = [];
  if (semitones !== 0) {
    // Resampling shifts pitch and speed together, as a bigger or smaller bell would.
    filters.push(`asetrate=${Math.round(SAMPLE_RATE * 2 ** (semitones / 12))}`, `aresample=${SAMPLE_RATE}`);
  }
  for (let s = speed; s > 1.0001; s /= Math.min(s, 2)) filters.push(`atempo=${Math.min(s, 2)}`);
  const args = ["-v", "error", "-ss", String(start), "-t", String(length), "-i", await recording(freesound), "-ac", "1", "-ar", String(SAMPLE_RATE)];
  if (filters.length > 0) args.push("-af", filters.join(","));
  // execFileSync rather than zx's $, which decodes output as text and would
  // corrupt the raw audio.
  const bytes = execFileSync("ffmpeg", [...args, "-f", "f32le", "-"], { maxBuffer: 1 << 28 });
  return new Float32Array(bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.length));
}

/** Mixes the layers, fades the ends and masters the result. */
async function build({ layers, fadeIn = 0.005, fadeOut = 0.05, drive = 2.5 }) {
  const parts = await Promise.all(layers.map(async (l) => ({ samples: await layer(l), at: l.at ?? 0, gain: l.gain ?? 1 })));
  const length = Math.max(...parts.map((p) => Math.round(p.at * SAMPLE_RATE) + p.samples.length));
  const mix = new Float32Array(length);
  for (const { samples, at, gain } of parts) {
    const offset = Math.round(at * SAMPLE_RATE);
    for (let i = 0; i < samples.length; i++) mix[offset + i] += samples[i] * gain;
  }
  const fadeInLength = Math.round(fadeIn * SAMPLE_RATE);
  const fadeOutLength = Math.round(fadeOut * SAMPLE_RATE);
  for (let i = 0; i < fadeInLength && i < length; i++) mix[i] *= i / fadeInLength;
  for (let i = 0; i < fadeOutLength && i < length; i++) mix[length - 1 - i] *= i / fadeOutLength;
  return master(mix, drive);
}

/** Soft-clips with tanh and scales to a 0.97 peak, for a loud, punchy sound. */
function master(samples, drive) {
  let peak = 0;
  for (const s of samples) peak = Math.max(peak, Math.abs(s));
  if (peak === 0) throw new Error("silent");
  let outPeak = 0;
  for (let i = 0; i < samples.length; i++) {
    samples[i] = Math.tanh((drive * samples[i]) / peak);
    outPeak = Math.max(outPeak, Math.abs(samples[i]));
  }
  for (let i = 0; i < samples.length; i++) samples[i] *= 0.97 / outPeak;
  return samples;
}

const credits = new Map();
const entries = [
  ...Object.entries(config.tracks).map(([track, spec]) => [track.padStart(3, "0"), spec]),
  ...Object.entries(config.extras),
];
for (const [file, spec] of entries) {
  const samples = await build(spec);
  await fs.writeFile(path.join(out, `${file}.wav`), encodeWav(samples));
  const ms = Math.round((samples.length / SAMPLE_RATE) * 1000);
  console.log(`sim/web/samples/${file}.wav  ${spec.name} (${ms} ms)`);
  for (const { freesound } of spec.layers) {
    if (!credits.has(freesound)) credits.set(freesound, new Set());
    credits.get(freesound).add(spec.name);
  }
}

const rows = [...credits].map(([id, uses]) => {
  const s = config.sources[id];
  return `| [${s.title}](${s.page}) | ${s.author} | ${s.licence} | ${[...uses].join(", ")} |`;
});
await fs.writeFile(
  path.join(root, "docs/sound-credits.md"),
  `# Sound credits

The recorded sounds in \`sim/web/samples/\` are cut from these recordings on
[Freesound](https://freesound.org). All are
[CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) (public domain), so
no credit is required. Thank you to everyone who shared them.

This file is generated from \`sim/samples/sources.json\` by
\`pnpm run import:samples\`; edit that file, not this one.

| Recording | By | Licence | Used for |
|---|---|---|---|
${rows.join("\n")}
`,
);
console.log(`docs/sound-credits.md  (${rows.length} recordings)`);
