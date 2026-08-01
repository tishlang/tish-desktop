#!/usr/bin/env node
/**
 * Run tish-format over every .tish file in the repo.
 *
 * `tish-format` takes exactly ONE <FILE> — it accepts neither directories nor multiple
 * paths. The npm scripts used to call `tish-format examples packages cli`, which the CLI
 * rejected with "error: unexpected argument 'packages' found" before formatting anything.
 * Because the CI step is continue-on-error, that failure was only ever an annotation, so
 * the formatter silently never ran and 52 of 73 files drifted.
 *
 * Usage:
 *   node scripts/format-tish.mjs            # format in place
 *   node scripts/format-tish.mjs --check    # report drift, exit 1 if any
 */
import { execFileSync, spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const ROOTS = ["examples", "packages", "cli"];
const check = process.argv.includes("--check");

// Prefer the pinned devDependency; fall back to npx for a bare checkout.
function resolveFormatter() {
  const local = path.join(ROOT, "node_modules", ".bin", "tish-format");
  if (fs.existsSync(local)) return { cmd: local, prefix: [] };
  return { cmd: "npx", prefix: ["--yes", "@tishlang/tish-format"] };
}

function collect(dir, out = []) {
  let entries;
  try {
    entries = fs.readdirSync(dir, { withFileTypes: true });
  } catch {
    return out;
  }
  for (const e of entries) {
    if (e.name === "node_modules" || e.name === "dist" || e.name.startsWith(".")) continue;
    const p = path.join(dir, e.name);
    if (e.isDirectory()) collect(p, out);
    else if (e.name.endsWith(".tish")) out.push(p);
  }
  return out;
}

const { cmd, prefix } = resolveFormatter();
const files = ROOTS.flatMap((r) => collect(path.join(ROOT, r))).sort();
if (files.length === 0) {
  console.error("no .tish files found — did the roots move?");
  process.exit(1);
}

const drifted = [];
for (const file of files) {
  const args = [...prefix, ...(check ? ["--check"] : []), file];
  const res = spawnSync(cmd, args, { cwd: ROOT, stdio: check ? "pipe" : "inherit" });
  if (res.status !== 0) {
    if (!check) {
      console.error(`tish-format failed on ${path.relative(ROOT, file)}`);
      process.exit(res.status ?? 1);
    }
    drifted.push(path.relative(ROOT, file));
  }
}

if (check) {
  if (drifted.length > 0) {
    console.error(`${drifted.length} of ${files.length} file(s) are not formatted:`);
    for (const f of drifted) console.error(`  ${f}`);
    console.error("\nRun: npm run format");
    process.exit(1);
  }
  console.log(`all ${files.length} .tish files are formatted`);
} else {
  console.log(`formatted ${files.length} .tish file(s)`);
}
