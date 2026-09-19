import fs from "node:fs";

export default {
  available: fs.existsSync(new URL("../assets/img/demo.mp4", import.meta.url)),
};
