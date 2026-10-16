// Writes the sounds to WAV files for the DFPlayer Mini's SD card. Each track
// uses its recording from sim/web/samples/ if it has one, otherwise the
// synthesised version from sim/web/sounds.js. Run with:
//   pnpm run render:sounds          writes target/sd-card/01/NNN.wav
//   pnpm run check:sounds           only checks every sound is usable
//
// Copy the `01` folder to the root of a FAT32-formatted microSD card.
import { argv, fs, path } from "zx";

import { BELL_SAMPLES, SOUNDS, render } from "./web/sounds.js";
import { SAMPLE_RATE, decodeWav, encodeWav } from "./wav.mjs";

const root = path.resolve(import.meta.dirname, "..");
const out = path.join(root, "target/sd-card/01");
const checkOnly = argv.check === true;
const problems = [];

/** Problems with `samples` as a sound that must fit in `durationMs`. */
function check(label, samples, sampleRate, durationMs) {
  const ms = (samples.length / sampleRate) * 1000;
  const peak = samples.reduce((max, s) => Math.max(max, Math.abs(s)), 0);
  if (samples.some(Number.isNaN)) problems.push(`${label}: contains NaN`);
  if (peak === 0) problems.push(`${label}: silent`);
  if (peak > 1) problems.push(`${label}: clips (peak ${peak.toFixed(2)})`);
  // The game assumes the sound has finished after durationMs.
  if (durationMs && ms > durationMs + 1) problems.push(`${label}: ${ms.toFixed(0)} ms is longer than durationMs ${durationMs}`);
  return ms;
}

async function readSample(file) {
  const full = path.join(root, "sim/web", file);
  if (!(await fs.pathExists(full))) {
    problems.push(`${file}: missing (run pnpm run import:samples)`);
    return undefined;
  }
  return decodeWav(await fs.readFile(full));
}

if (!checkOnly) await fs.ensureDir(out);

for (const [track, { name, durationMs, sample }] of Object.entries(SOUNDS)) {
  const synth = render(Number(track), SAMPLE_RATE);
  check(`${track} ${name} (synthesised)`, synth, SAMPLE_RATE, durationMs);

  let chosen = synth;
  let kind = "synthesised";
  if (sample) {
    const recorded = await readSample(sample);
    if (recorded) {
      check(`${track} ${name} (${sample})`, recorded.samples, recorded.sampleRate, durationMs);
      chosen = recorded.samples;
      kind = "recorded";
    }
  }

  if (!checkOnly) {
    const file = path.join(out, `${track.padStart(3, "0")}.wav`);
    await fs.writeFile(file, encodeWav(chosen));
    const ms = (chosen.length / SAMPLE_RATE) * 1000;
    console.log(`${path.relative(root, file)}  ${name}, ${kind} (${ms.toFixed(0)} ms)`);
  }
}

for (const sample of BELL_SAMPLES) {
  const bell = await readSample(sample);
  if (bell) check(sample, bell.samples, bell.sampleRate);
}

if (problems.length > 0) {
  for (const problem of problems) console.log(`::error::${problem}`);
  process.exitCode = 1;
} else {
  console.log(`OK: ${Object.keys(SOUNDS).length} sounds and ${BELL_SAMPLES.length} bells are usable.`);
}
