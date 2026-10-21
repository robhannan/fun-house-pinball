// Installs the latest OpenSCAD (the nightly build) on the Ubuntu CI runner,
// from OpenSCAD's own repository on the openSUSE Build Service. Ubuntu's own
// package is years old. Run with: pnpm run ci:install-openscad
import { tmpdir } from "node:os";

import { $, path } from "zx";

const release = (await $`lsb_release -rs`).stdout.trim();
const repo = `https://download.opensuse.org/repositories/home:/t-paul/xUbuntu_${release}`;
const key = path.join(tmpdir(), "openscad.gpg");

await $`curl -fsSL ${`${repo}/Release.key`}`.pipe($`gpg --dearmor --yes -o ${key}`);
await $`sudo install -D -m 644 ${key} /etc/apt/keyrings/openscad.gpg`;
await $`echo ${`deb [signed-by=/etc/apt/keyrings/openscad.gpg] ${repo}/ /`}`.pipe($`sudo tee /etc/apt/sources.list.d/openscad.list`);
await $`sudo apt-get update -q`;
await $`sudo apt-get install -y -q --no-install-recommends openscad-nightly`;
await $`openscad-nightly --version`;
