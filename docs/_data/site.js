import fs from "node:fs";

const cargo = fs.readFileSync(new URL("../../Cargo.toml", import.meta.url), "utf8");
const version = cargo.match(/^version\s*=\s*"([^"]+)"/m)?.[1] ?? "0.0.0";

export default {
  name: "mido",
  tagline: "Markdown In, Document Out.",
  description:
    "A terminal Markdown reader that renders documents the way a good reader app does: real heading hierarchy, a comfortable line measure, syntax-highlighted code, and tables that line up.",
  url: "https://anistark.github.io/mido",
  repo: "https://github.com/anistark/mido",
  x: "https://x.com/kranirudha",
  version,
};
