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

import { BELL_SAMPLES, COIL_SAMPLES, SOUNDS, render } from "./sounds.js";

const PHASES = ["Attract", "Playing", "Tilted", "Test"];

/// WebAssembly exports this script needs; an older build lacks some of them.
const REQUIRED_EXPORTS = [
  "sim_advance",
  "sim_ball_radius",
  "sim_ball_x",
  "sim_ball_y",
  "sim_ball_shown",
  "sim_flipper_turn",
  "sim_plunger",
  "sim_plunger_pull",
  "sim_nudge",
  "sim_test_reports",
  "sim_trapdoor_open",
];
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
  // Never use cached copies: after a rebuild, the page, this script and the
  // WebAssembly must all be from the same build.
  const get = (file) => fetch(file, { cache: "no-store" });
  const [wasmBytes, playfield, backboard] = await Promise.all([
    get("generated/pinball_sim.wasm").then((r) => r.arrayBuffer()),
    get("generated/playfield.svg").then((r) => r.text()),
    get("generated/backboard.svg").then((r) => r.text()),
  ]);
  const { instance } = await WebAssembly.instantiate(wasmBytes, {});
  const missing = REQUIRED_EXPORTS.filter((name) => typeof instance.exports[name] !== "function");
  if (missing.length > 0) {
    throw new Error(
      `The simulator build is out of date (missing ${missing.join(", ")}). Run pnpm run build:sim, then reload.`,
    );
  }
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
  const flippers = [...document.querySelectorAll("#playfield [data-flipper]")].map((el) => {
    // Turn about the pivot: the middle of the base end (the first and last
    // points of the path), as the physics does.
    const p = el.getAttribute("d").match(/-?[\d.]+/g).map(Number);
    return { el, n: el.dataset.flipper === "left" ? 0 : 1, pivot: [(p[0] + p[6]) / 2, (p[1] + p[7]) / 2] };
  });

  // The trapdoors glow while their flaps are open.
  const trapdoors = [...document.querySelectorAll("#playfield [data-trapdoor]")];

  // The ball, drawn on top of the playfield.
  const svg = document.querySelector("#playfield svg");
  const ballEl = document.createElementNS("http://www.w3.org/2000/svg", "circle");
  ballEl.id = "sim-ball";
  ballEl.setAttribute("r", String(sim.sim_ball_radius()));
  ballEl.setAttribute("fill", "#b8c0c8");
  ballEl.setAttribute("stroke", "#4b535b");
  ballEl.setAttribute("stroke-width", "2");
  svg.appendChild(ballEl);

  // Ball controls that aren't switches, by their buttons' data-action: the
  // plunger (hold to pull back, let go to launch) and the nudge.
  const pullMeter = document.getElementById("plunger-pull");
  const actions = {
    plunger: (down) => sim.sim_plunger(down ? 1 : 0),
    nudge: (down) => down && sim.sim_nudge(),
  };
  for (const button of document.querySelectorAll("button[data-action]")) {
    const act = actions[button.dataset.action];
    if (button.hasAttribute("data-hold")) {
      const release = () => {
        act(false);
        button.classList.remove("pressed");
      };
      button.addEventListener("pointerdown", () => {
        act(true);
        button.classList.add("pressed");
      });
      button.addEventListener("pointerup", release);
      button.addEventListener("pointerleave", release);
    } else {
      button.addEventListener("click", () => act(true));
    }
  }

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
    const music = document.getElementById("attract-music").checked ? 1 : 0;
    audio.stopAll();
    sim.sim_reset_with(balls, replays, bells, music);
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
    button.addEventListener("click", () => sim.sim_set_trapdoor(n, sim.sim_trapdoor_open(n) ? 0 : 1));
  }

  // Keyboard shortcuts come from the buttons' data-key attributes, so the
  // labels live in one place (index.html, which the Rust tests check).
  const keys = new Map(
    [...document.querySelectorAll("button[data-key]")].map((button) => [button.dataset.key, button]),
  );
  // A button either works a switch or does an action.
  const hold = (button, down) =>
    button.dataset.action ? actions[button.dataset.action](down) : press(button.dataset.switch, down);
  const held = new Set();
  const keyButton = (event) =>
    event.ctrlKey || event.metaKey || event.altKey ? undefined : keys.get(event.key.toLowerCase());

  addEventListener("keydown", (event) => {
    const button = keyButton(event);
    if (!button) return;
    event.preventDefault();
    if (event.repeat) return;
    if (button.hasAttribute("data-hold")) {
      hold(button, true);
      held.add(button);
    } else if (button.dataset.action) {
      actions[button.dataset.action](true);
    } else {
      tap(button.dataset.switch);
    }
    button.classList.add("pressed");
  });
  const release = (button) => {
    if (held.delete(button)) hold(button, false);
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

  let testReports = 0;
  const text = (id, value) => {
    const el = document.getElementById(id);
    if (el.textContent !== value) el.textContent = value;
  };

  const coilLastOn = {};
  let last = performance.now();
  let carry = 0;
  function frame(now) {
    carry += now - last;
    last = now;
    const ms = Math.floor(carry);
    carry -= ms;
    if (ms > 0) sim.sim_advance(ms);

    const coilsSeen = sim.sim_coils_seen() >>> 0;
    const lampLow = sim.sim_lamps_low() >>> 0;
    const lampHigh = sim.sim_lamps_high() >>> 0;

    for (const { el, lamp } of lamps) {
      const word = lamp < 32 ? lampLow : lampHigh;
      el.classList.toggle("lit", ((word >>> (lamp % 32)) & 1) === 1);
    }
    for (const n of Object.keys(COIL_SAMPLES)) {
      if (coilsSeen & (1 << n)) {
        if (now - (coilLastOn[n] ?? -Infinity) > 60) audio.coil(n);
        coilLastOn[n] = now;
      }
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
    for (const { el, n, pivot } of flippers) {
      const degrees = (sim.sim_flipper_turn(n) * 180) / Math.PI;
      el.setAttribute("transform", `rotate(${degrees.toFixed(2)} ${pivot[0]} ${pivot[1]})`);
    }
    for (const el of trapdoors) el.classList.toggle("open", sim.sim_trapdoor_open(Number(el.dataset.trapdoor)) === 1);
    ballEl.setAttribute("cx", sim.sim_ball_x().toFixed(1));
    ballEl.setAttribute("cy", sim.sim_ball_y().toFixed(1));
    ballEl.style.display = sim.sim_ball_shown() ? "" : "none";
    pullMeter.style.width = `${Math.round(sim.sim_plunger_pull() * 100)}%`;

    for (let track = sim.sim_take_stop(); track !== 0; track = sim.sim_take_stop()) audio.stop(track);
    for (let sound = sim.sim_take_sound(); sound !== 0; sound = sim.sim_take_sound()) {
      audio.play(sound & 0xff, (sound & 0x100) !== 0);
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

    // Test-mode reports: the controller's serial console, newest first.
    const reports = sim.sim_test_reports();
    if (reports !== testReports) {
      testReports = reports;
      const lines = [];
      for (let n = 0; ; n++) {
        const len = sim.sim_test_line_len(n);
        if (len === 0) break;
        let line = "";
        for (let i = 0; i < len; i++) line += String.fromCharCode(sim.sim_test_line_byte(n, i));
        lines.push(line);
      }
      text("test-log", lines.join("\n") || "–");
    }

    requestAnimationFrame(frame);
  }
  requestAnimationFrame(frame);
}

load().then(start, (error) => {
  document.getElementById("playfield").textContent =
    `Couldn't load the simulator: ${error.message} (Build it with pnpm run build:sim and serve it with pnpm run serve:sim.)`;
});

/**
 * Plays sounds with the Web Audio API, mixing them like the WAV Trigger: up to
 * VOICES at once, and when all are busy the oldest unlocked sound is stopped to
 * make room. Like the SD-card files, each sound is played with HEADROOM so that
 * overlapping sounds don't clip, then a limiter brings the mix back up. Each sound can be the recorded
 * version (from free recordings) or the synthesised one, picked on the page.
 * Real bells ring separately, as real bells would. Browsers only allow audio
 * after the user has interacted with the page, so the audio context is created
 * suspended (the recordings load meanwhile) and resumes on the first click or
 * key press.
 */
/** Sounds the WAV Trigger can mix at once. */
const VOICES = 14;
/** Matches the SD-card files (sim/render-sounds.mjs). */
const HEADROOM = 0.5;

function soundPlayer() {
  let context;
  let gain;
  /** Playing sounds, oldest first: { source, locked, track }. */
  let voices = [];
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
      // A limiter, then make-up gain for the headroom left in each sound.
      const limiter = context.createDynamicsCompressor();
      limiter.threshold.value = -6;
      limiter.knee.value = 3;
      limiter.ratio.value = 20;
      limiter.attack.value = 0.002;
      limiter.release.value = 0.1;
      const makeUp = context.createGain();
      makeUp.gain.value = 1 / HEADROOM;
      gain.connect(limiter).connect(makeUp).connect(context.destination);
      for (const [track, { sample }] of Object.entries(SOUNDS)) if (sample) loadRecording(track, sample);
      BELL_SAMPLES.forEach((file, n) => loadRecording(`bell-${n}`, file));
      for (const file of new Set(Object.values(COIL_SAMPLES))) loadRecording(file, file);
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

  const start = (audioBuffer, level = 1) => {
    const source = context.createBufferSource();
    source.buffer = audioBuffer;
    if (level === 1) {
      source.connect(gain);
    } else {
      const quieter = context.createGain();
      quieter.gain.value = level;
      source.connect(quieter).connect(gain);
    }
    source.start();
    return source;
  };

  const bufferFor = (track) => {
    // The attract music has no synthesised stand-in: the lamps follow the
    // recording's beats, so anything else would be out of step. If it hasn't
    // loaded yet, the lamps dance in silence.
    if (SOUNDS[track].recordingOnly) return recorded.get(String(track));
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

  /** Plays `track`, mixed with whatever is playing. */
  const play = (track, locked = false) => {
    if (!on.checked || !context || !SOUNDS[track]) return;
    if (voices.length >= VOICES) {
      const steal = voices.findIndex((voice) => !voice.locked);
      if (steal < 0) return; // Every voice is locked: drop this one.
      voices[steal].source.stop();
      voices.splice(steal, 1);
    }
    const audioBuffer = bufferFor(track);
    if (!audioBuffer) return;
    const voice = { source: start(audioBuffer, HEADROOM), locked, track };
    voice.source.onended = () => {
      voices = voices.filter((v) => v !== voice);
    };
    voices.push(voice);
  };

  /** Stops every playing copy of `track`, as the WAV Trigger's STOP does. */
  const stop = (track) => {
    for (const voice of voices.filter((v) => v.track === track)) voice.source.stop();
  };
  const stopAll = () => {
    for (const voice of voices) voice.source.stop();
  };

  /** Rings real bell `n` (0 = 1s, 1 = 10s, 2 = 100s), over whatever is playing. */
  const bell = (n) => {
    const audioBuffer = context && recorded.get(`bell-${n}`);
    if (on.checked && audioBuffer) start(audioBuffer, HEADROOM);
  };

  /**
   * The noise coil `n` makes when it fires (kickers, pop bumpers, flippers). On
   * the real machine the coils make it themselves, so only the simulator plays
   * it, quieter than the effects and over whatever is playing.
   */
  const coilNoises = document.getElementById("coil-noises");
  const coil = (n) => {
    const audioBuffer = context && recorded.get(COIL_SAMPLES[n]);
    if (on.checked && coilNoises.checked && audioBuffer) start(audioBuffer, 0.45 * HEADROOM);
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
  addButton("Coil noise: kicker", () => coil(1));
  addButton("Coil noise: pop bumper", () => coil(3));
  addButton("Coil noise: flipper", () => coil(8));

  return { play, stop, stopAll, bell, coil };
}
