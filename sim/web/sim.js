// Browser front end for crates/pinball-sim. Loads the WebAssembly build and the
// playfield and backboard SVGs (all copied into generated/ by
// `pnpm run build:sim`), then:
//
// - turns clicks and key presses into switch closures
// - runs the simulated machine in step with real time
// - lights lamps, flashes coils and raises flippers from the machine's outputs
// - turns the backboard's score reels and credit window
// - plays the machine's sounds (synthesised in sounds.js)
//
// Parts are found by their data-switch / data-coil / data-lamp attributes, and
// reels by data-score / data-digit. The Rust tests check them against
// playfield.rs.

import { BELL_SAMPLES, SOUNDS, render } from "./sounds.js";

const PHASES = ["Attract", "Playing", "Tilted"];
const COIL_GLOW_MS = 150; // keep short coil pulses visible

// "C2R3" (matrix column 2, row 3) -> 19; "D1" (direct input 1) -> 65.
function switchIndex(label) {
  const matrix = /^C(\d)R(\d)$/.exec(label);
  if (matrix) return Number(matrix[1]) * 8 + Number(matrix[2]);
  const direct = /^D(\d)$/.exec(label);
  if (direct) return 64 + Number(direct[1]);
  throw new Error(`bad switch label ${label}`);
}

async function load() {
  const [wasmBytes, playfield, backboard] = await Promise.all([
    fetch("generated/pinball_sim.wasm").then((r) => r.arrayBuffer()),
    fetch("generated/playfield.svg").then((r) => r.text()),
    fetch("generated/backboard.svg").then((r) => r.text()),
  ]);
  const { instance } = await WebAssembly.instantiate(wasmBytes, {});
  document.getElementById("playfield").innerHTML = playfield;
  document.getElementById("backboard").innerHTML = backboard;
  return instance.exports;
}

function start(sim) {
  const lamps = [...document.querySelectorAll("[data-lamp]")].map((el) => ({
    el,
    lamp: Number(el.dataset.lamp),
  }));
  const coils = [...document.querySelectorAll("[data-coil]")].map((el) => ({
    el,
    mask: el.dataset.coil.split(" ").reduce((m, c) => m | (1 << Number(c)), 0),
    lastOn: -Infinity,
  }));
  const flippers = coils.filter(({ el }) => el.dataset.flipper);

  const press = (label, closed) => sim.sim_set_switch(switchIndex(label), closed ? 1 : 0);
  const tap = (label) => sim.sim_tap(switchIndex(label));

  // Playfield parts: a click is a ball rolling over the switch.
  for (const el of document.querySelectorAll("#playfield [data-switch]")) {
    el.addEventListener("pointerdown", (event) => {
      event.preventDefault();
      tap(el.dataset.switch);
      el.classList.add("hit");
      setTimeout(() => el.classList.remove("hit"), 120);
    });
  }

  // Cabinet buttons: flippers are held, the rest are pressed briefly.
  for (const button of document.querySelectorAll("button[data-switch]")) {
    const label = button.dataset.switch;
    if (button.hasAttribute("data-hold")) {
      const release = () => press(label, false);
      button.addEventListener("pointerdown", () => press(label, true));
      button.addEventListener("pointerup", release);
      button.addEventListener("pointerleave", release);
      button.addEventListener("pointercancel", release);
    } else {
      button.addEventListener("click", () => tap(label));
    }
  }
  // Operator settings take effect on a power cycle, as on the real machine.
  const powerCycle = () => {
    const balls = Number(document.getElementById("balls").value);
    const replays = document.getElementById("replay-mode").checked ? 1 : 0;
    const bells = document.getElementById("bells-fitted").checked ? 1 : 0;
    sim.sim_reset_with(balls, replays, bells);
  };
  document.getElementById("reset").addEventListener("click", powerCycle);
  document.getElementById("apply-settings").addEventListener("click", powerCycle);

  // Backboard: three reels per player (hundreds, tens, units), which wrap at
  // 1000 like the real ones, and the two-digit credit window.
  const reels = [...document.querySelectorAll("#backboard [data-score]")].map((el) => ({
    el,
    player: Number(el.dataset.score),
    place: 10 ** (2 - Number(el.dataset.digit)),
  }));
  const credits = document.querySelector("#backboard [data-credits]");

  // Trapdoor flaps: toggle by hand until the rule that opens them is known.
  for (const button of document.querySelectorAll("button[data-trapdoor-toggle]")) {
    const n = Number(button.dataset.trapdoorToggle);
    const coil = Number(document.querySelector(`[data-trapdoor="${n}"]`).dataset.coil);
    button.addEventListener("click", () => {
      const open = (sim.sim_coils() >>> coil) & 1;
      sim.sim_set_trapdoor(n, open ? 0 : 1);
    });
  }

  // Keyboard shortcuts come from the buttons' data-key attributes, so the
  // labels live in one place (index.html, which the Rust tests check).
  const keys = new Map(
    [...document.querySelectorAll("button[data-key]")].map((button) => [button.dataset.key, button]),
  );
  const held = new Set();
  const keyButton = (event) =>
    event.ctrlKey || event.metaKey || event.altKey ? undefined : keys.get(event.key.toLowerCase());

  addEventListener("keydown", (event) => {
    const button = keyButton(event);
    if (!button) return;
    event.preventDefault();
    if (event.repeat) return;
    if (button.hasAttribute("data-hold")) {
      press(button.dataset.switch, true);
      held.add(button);
    } else {
      tap(button.dataset.switch);
    }
    button.classList.add("pressed");
  });
  const release = (button) => {
    if (held.delete(button)) press(button.dataset.switch, false);
    button.classList.remove("pressed");
  };
  addEventListener("keyup", (event) => {
    const button = keys.get(event.key.toLowerCase());
    if (button) release(button);
  });
  // A key released while the window is in the background never sends keyup,
  // so let go of everything when focus is lost.
  addEventListener("blur", () => {
    for (const button of [...held]) release(button);
  });

  const audio = soundPlayer();

  const text = (id, value) => {
    const el = document.getElementById(id);
    if (el.textContent !== value) el.textContent = value;
  };

  let last = performance.now();
  let carry = 0;
  function frame(now) {
    carry += now - last;
    last = now;
    const ms = Math.floor(carry);
    carry -= ms;
    if (ms > 0) sim.sim_advance(ms);

    const coilBits = sim.sim_coils() >>> 0;
    const coilsSeen = sim.sim_coils_seen() >>> 0;
    const lampLow = sim.sim_lamps_low() >>> 0;
    const lampHigh = sim.sim_lamps_high() >>> 0;

    for (const { el, lamp } of lamps) {
      const word = lamp < 32 ? lampLow : lampHigh;
      el.classList.toggle("lit", ((word >>> (lamp % 32)) & 1) === 1);
    }
    for (const coil of coils) {
      if (coilsSeen & coil.mask) {
        // A bell stroke: each new pulse rings the bell.
        if (coil.el.classList.contains("bell") && now - coil.lastOn > 40) audio.bell(Number(coil.el.dataset.coil) - 15);
        coil.lastOn = now;
      }
      if (!coil.el.dataset.flipper) {
        coil.el.classList.toggle("firing", now - coil.lastOn < COIL_GLOW_MS);
      }
    }
    for (const { el, mask } of flippers) el.classList.toggle("up", (coilBits & mask) !== 0);

    for (let track = sim.sim_take_sound(); track !== 0; track = sim.sim_take_sound()) {
      audio.play(track);
    }

    const ball = sim.sim_ball();
    for (const { el, player, place } of reels) {
      const digit = String(Math.floor(sim.sim_score_of(player) / place) % 10);
      if (el.textContent !== digit) el.textContent = digit;
    }
    const creditText = String(sim.sim_credits()).padStart(2, "0");
    if (credits.textContent !== creditText) credits.textContent = creditText;

    const player = sim.sim_current_player();
    text("player", player === 0 ? "–" : String(player));
    text("players", String(sim.sim_players()));
    text("ball", ball === 0 ? "–" : `${ball} of ${sim.sim_balls_per_game()}`);
    text("replays", String(sim.sim_replays()));
    text("bonus", String(sim.sim_bonus()));
    text("phase", PHASES[sim.sim_phase()] ?? "?");

    requestAnimationFrame(frame);
  }
  requestAnimationFrame(frame);
}

load().then(start, (error) => {
  document.getElementById("playfield").textContent =
    `Couldn't load the simulator (${error.message}). Run pnpm run build:sim, then serve sim/web/ over HTTP.`;
});

/**
 * Plays sounds with the Web Audio API. Like the DFPlayer Mini, only one sound
 * plays at a time: a new one stops the last. Each sound can be the recorded
 * version (from free recordings) or the synthesised one, picked on the page.
 * Real bells ring separately, as real bells would. Browsers only allow audio
 * after the user has interacted with the page, so the audio context is created
 * suspended (the recordings load meanwhile) and resumes on the first click or
 * key press.
 */
function soundPlayer() {
  let context;
  let gain;
  let current;
  const synthesised = new Map();
  const recorded = new Map();
  const on = document.getElementById("sound-on");
  const volume = document.getElementById("volume");
  const style = document.getElementById("sound-style");

  const loadRecording = async (key, file) => {
    try {
      const bytes = await (await fetch(file)).arrayBuffer();
      recorded.set(key, await context.decodeAudioData(bytes));
    } catch (error) {
      console.warn(`couldn't load ${file}; using the synthesised sound`, error);
    }
  };

  const ensureContext = () => {
    if (!context) {
      context = new AudioContext();
      gain = context.createGain();
      gain.gain.value = Number(volume.value);
      gain.connect(context.destination);
      for (const [track, { sample }] of Object.entries(SOUNDS)) if (sample) loadRecording(track, sample);
      BELL_SAMPLES.forEach((file, n) => loadRecording(`bell-${n}`, file));
    }
    if (context.state === "suspended") context.resume();
    return context;
  };
  // Create the (suspended) context now, so the recordings load straight away;
  // the first click or key press resumes it.
  ensureContext();
  for (const event of ["pointerdown", "keydown"]) addEventListener(event, ensureContext, { once: true });
  volume.addEventListener("input", () => {
    if (gain) gain.gain.value = Number(volume.value);
  });

  const start = (audioBuffer) => {
    const source = context.createBufferSource();
    source.buffer = audioBuffer;
    source.connect(gain);
    source.start();
    return source;
  };

  const bufferFor = (track) => {
    if (style.value === "recorded" && recorded.has(String(track))) return recorded.get(String(track));
    let audioBuffer = synthesised.get(track);
    if (!audioBuffer) {
      const samples = render(track, context.sampleRate);
      audioBuffer = context.createBuffer(1, samples.length, context.sampleRate);
      audioBuffer.copyToChannel(samples, 0);
      synthesised.set(track, audioBuffer);
    }
    return audioBuffer;
  };

  const play = (track) => {
    if (!on.checked || !context || !SOUNDS[track]) return;
    current?.stop();
    current = start(bufferFor(track));
  };

  /** Rings real bell `n` (0 = 1s, 1 = 10s, 2 = 100s), over whatever is playing. */
  const bell = (n) => {
    const audioBuffer = context && recorded.get(`bell-${n}`);
    if (on.checked && audioBuffer) start(audioBuffer);
  };

  // The sound-test board: one button per sound, plus the bells.
  const board = document.getElementById("sound-test");
  const addButton = (label, action) => {
    const button = document.createElement("button");
    button.textContent = label;
    button.addEventListener("click", () => {
      ensureContext();
      action();
    });
    board.append(button);
  };
  for (const [track, { name }] of Object.entries(SOUNDS)) addButton(name, () => play(Number(track)));
  ["1s", "10s", "100s"].forEach((label, n) => addButton(`Real bell: ${label}`, () => bell(n)));

  return { play, bell };
}
