// Browser front end for crates/pinball-sim. Loads the WebAssembly build and the
// playfield SVG (both copied into generated/ by `pnpm run build:sim`), then:
//
// - turns clicks and key presses into switch closures
// - runs the simulated machine in step with real time
// - lights lamps, flashes coils and raises flippers from the machine's outputs
//
// Parts are found by their data-switch / data-coil / data-lamp attributes. The
// Rust tests check those attributes against playfield.rs.

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
  const [wasmBytes, svgText] = await Promise.all([
    fetch("generated/pinball_sim.wasm").then((r) => r.arrayBuffer()),
    fetch("generated/playfield.svg").then((r) => r.text()),
  ]);
  const { instance } = await WebAssembly.instantiate(wasmBytes, {});
  document.getElementById("playfield").innerHTML = svgText;
  return instance.exports;
}

function start(sim) {
  const lamps = [...document.querySelectorAll("[data-lamp]")].map((el) => ({
    el,
    lamp: Number(el.dataset.lamp),
  }));
  const coils = [...document.querySelectorAll("#playfield [data-coil]")].map((el) => ({
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
  document.getElementById("reset").addEventListener("click", () => sim.sim_reset());

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
      if (coilsSeen & coil.mask) coil.lastOn = now;
      if (!coil.el.dataset.flipper) {
        coil.el.classList.toggle("firing", now - coil.lastOn < COIL_GLOW_MS);
      }
    }
    for (const { el, mask } of flippers) el.classList.toggle("up", (coilBits & mask) !== 0);

    const ball = sim.sim_ball();
    text("score", String(sim.sim_score()));
    text("ball", ball === 0 ? "–" : String(ball));
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
