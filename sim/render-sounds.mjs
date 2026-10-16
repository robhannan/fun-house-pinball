// Renders the sounds in sim/web/sounds.js to WAV files for the DFPlayer Mini's
// SD card. Run with:
//   pnpm run render:sounds          writes target/sd-card/01/NNN.wav
//   pnpm run check:sounds           only checks every sound renders cleanly
//
// Copy the `01` folder to the root of a FAT32-formatted microSD card.
import { fs, path, argv } from "zx";

import { SOUNDS, render } from "./web/sounds.js";

const SAMPLE_RATE = 44_100;
const root = path.resolve(import.meta.dirname, "..");
const out = path.join(root, "target/sd-card/01");
const checkOnly = argv.check === true;

/** 16-bit mono PCM WAV. */
function wav(samples) {
  const data = Buffer.alloc(samples.length * 2);
  samples.forEach((s, i) => data.writeInt16LE(Math.round(Math.max(-1, Math.min(1, s)) * 32767), i * 2));
  const header = Buffer.alloc(44);
  header.write("RIFF", 0);
  header.writeUInt32LE(36 + data.length, 4);
  header.write("WAVEfmt ", 8);
  header.writeUInt32LE(16, 16); // fmt chunk size
  header.writeUInt16LE(1, 20); // PCM
  header.writeUInt16LE(1, 22); // mono
  header.writeUInt32LE(SAMPLE_RATE, 24);
  header.writeUInt32LE(SAMPLE_RATE * 2, 28); // byte rate
  header.writeUInt16LE(2, 32); // block align
  header.writeUInt16LE(16, 34); // bits per sample
  header.write("data", 36);
  header.writeUInt32LE(data.length, 40);
  return Buffer.concat([header, data]);
}

const problems = [];
if (!checkOnly) await fs.ensureDir(out);

for (const [track, { name, durationMs }] of Object.entries(SOUNDS)) {
  const samples = render(Number(track), SAMPLE_RATE);
  const ms = (samples.length / SAMPLE_RATE) * 1000;
  const peak = samples.reduce((max, s) => Math.max(max, Math.abs(s)), 0);
  if (samples.some(Number.isNaN)) problems.push(`${track} ${name}: contains NaN`);
  if (peak === 0) problems.push(`${track} ${name}: silent`);
  if (peak > 1) problems.push(`${track} ${name}: clips (peak ${peak.toFixed(2)})`);
  // The game assumes the sound has finished after durationMs.
  if (ms > durationMs + 1) problems.push(`${track} ${name}: ${ms.toFixed(0)} ms is longer than durationMs ${durationMs}`);

  if (!checkOnly) {
    const file = path.join(out, `${track.padStart(3, "0")}.wav`);
    await fs.writeFile(file, wav(samples));
    console.log(`${path.relative(root, file)}  ${name} (${ms.toFixed(0)} ms)`);
  }
}

if (problems.length > 0) {
  for (const problem of problems) console.log(`::error::${problem}`);
  process.exitCode = 1;
} else {
  console.log(`OK: ${Object.keys(SOUNDS).length} sounds render cleanly.`);
}
