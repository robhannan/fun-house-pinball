// CI commands shared by the independent jobs in .github/workflows/ci.yml.
// Keep commands here so workflow run steps are single pnpm run calls.
import { $, path } from "zx";

const root = path.resolve(import.meta.dirname, "../..");
const task = process.argv[2];
$.verbose = true;
$.cwd = task === "firmware" || task === "firmware-toolchain" ? path.join(root, "firmware") : root;

switch (task) {
  case "toolchain":
  case "firmware-toolchain":
    // rustup reads the toolchain file in the selected project directory.
    if ((await $({ nothrow: true })`rustup show active-toolchain`).exitCode !== 0) {
      await $`rustup toolchain install`;
    }
    break;
  case "core":
    await $`cargo fmt --all --check`;
    await $`cargo clippy --all-targets --locked -- -D warnings`;
    await $`cargo test --locked`;
    break;
  case "firmware":
    await $`cargo fmt --check`;
    await $`cargo clippy --locked -- -D warnings`;
    await $`cargo build --release --locked`;
    await $`avr-size --format=avr --mcu=atmega2560 target/avr-none/release/fun-house-firmware.elf`;
    break;
  case "install-avr":
  case "install-ffmpeg": {
    const packages = task === "install-avr" ? ["gcc-avr", "avr-libc", "binutils-avr"] : ["ffmpeg"];
    await $`sudo apt-get update`;
    await $`sudo apt-get install -y --no-install-recommends ${packages}`;
    break;
  }
  default:
    throw new Error(`Unknown CI task: ${task}`);
}
