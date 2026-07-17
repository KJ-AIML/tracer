import { copyFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const src = join(root, "src", "styles.css");
const dest = join(root, "dist", "styles.css");
mkdirSync(dirname(dest), { recursive: true });
copyFileSync(src, dest);
console.log("copied styles.css → dist/styles.css");
