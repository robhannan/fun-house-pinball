// Reads and writes 16-bit PCM WAV files: mono for the simulator's samples,
// stereo for the WAV Trigger's SD card (which only plays 16-bit 44.1 kHz
// stereo, with no metadata before the audio).

export const SAMPLE_RATE = 44_100;

/** Encodes `samples` (-1..1) as a 16-bit WAV file, copied to `channels` channels. */
export function encodeWav(samples, sampleRate = SAMPLE_RATE, channels = 1) {
  const data = Buffer.alloc(samples.length * 2 * channels);
  samples.forEach((s, i) => {
    const value = Math.round(Math.max(-1, Math.min(1, s)) * 32767);
    for (let c = 0; c < channels; c++) data.writeInt16LE(value, (i * channels + c) * 2);
  });
  const header = Buffer.alloc(44);
  header.write("RIFF", 0);
  header.writeUInt32LE(36 + data.length, 4);
  header.write("WAVEfmt ", 8);
  header.writeUInt32LE(16, 16); // fmt chunk size
  header.writeUInt16LE(1, 20); // PCM
  header.writeUInt16LE(channels, 22);
  header.writeUInt32LE(sampleRate, 24);
  header.writeUInt32LE(sampleRate * 2 * channels, 28); // byte rate
  header.writeUInt16LE(2 * channels, 32); // block align
  header.writeUInt16LE(16, 34); // bits per sample
  header.write("data", 36);
  header.writeUInt32LE(data.length, 40);
  return Buffer.concat([header, data]);
}

/** Decodes a 16-bit mono WAV file written by `encodeWav`. */
export function decodeWav(buffer) {
  if (buffer.toString("ascii", 0, 4) !== "RIFF" || buffer.toString("ascii", 8, 12) !== "WAVE") {
    throw new Error("not a WAV file");
  }
  const sampleRate = buffer.readUInt32LE(24);
  const channels = buffer.readUInt16LE(22);
  const bits = buffer.readUInt16LE(34);
  if (channels !== 1 || bits !== 16) throw new Error(`expected 16-bit mono, got ${bits}-bit ${channels}-channel`);
  const length = buffer.readUInt32LE(40) / 2;
  const samples = new Float32Array(length);
  for (let i = 0; i < length; i++) samples[i] = buffer.readInt16LE(44 + i * 2) / 32768;
  return { sampleRate, samples };
}
