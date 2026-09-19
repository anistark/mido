import fs from "node:fs";

export default () =>
  fs
    .readFileSync(new URL("../../CHANGELOG.md", import.meta.url), "utf8")
    .replace(/^# Changelog\s*\n/, "");
