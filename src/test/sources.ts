// Node's types by name: TypeScript 6 no longer pulls every @types in, and
// tsconfig.json keeps the app code without them.
/// <reference types="node" />
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, resolve } from "node:path";

/* The source files as text, read from the disk at the moment of the test,
 * for the guards that look at every file (strict CSP, reduced motion, the
 * color-scheme and the glass tokens, the widget contract fixtures). Never
 * `import.meta.glob`: Vitest's module cache freezes its list of files for
 * the session, so a file added locally stayed invisible to the guards
 * until a restart (or `--no-fs-module-cache`). sources.test.ts keeps this
 * reader honest with a file created during the test. */

/** The repository root (the folder of package.json). */
export const ROOT = resolve(import.meta.dirname, "../..");

const SKIPPED = new Set(["node_modules", "target", "dist"]);

/** Every file under `folder` (relative to the root), as paths from the
 * root with `/` separators and a leading `/`: `/src/dock/Dock.tsx`.
 * `node_modules`, `target` and `dist` are skipped. */
export function listFiles(folder: string): string[] {
  const found: string[] = [];
  const walk = (dir: string, prefix: string) => {
    for (const name of readdirSync(dir).sort()) {
      if (SKIPPED.has(name)) continue;
      const path = join(dir, name);
      const key = `${prefix}/${name}`;
      if (statSync(path).isDirectory()) walk(path, key);
      else found.push(key);
    }
  };
  walk(resolve(ROOT, folder), `/${folder.replace(/\\/g, "/")}`);
  return found;
}

/** The text of the file at `path` (a key of `listFiles`, or a path from
 * the root). */
export function readSource(path: string): string {
  const relative = path.startsWith("/") ? path.slice(1) : path;
  return readFileSync(resolve(ROOT, relative), "utf8");
}

/** The files under `folder` whose path matches `match`, keyed as
 * `listFiles` does, with their text. Test files (`*.test.ts[x]`) are left
 * out unless `tests` is true. */
export function readSources(
  folder: string,
  match: RegExp,
  { tests = false } = {},
): Record<string, string> {
  const sources: Record<string, string> = {};
  for (const path of listFiles(folder)) {
    if (!match.test(path)) continue;
    if (!tests && /\.test\.tsx?$/.test(path)) continue;
    sources[path] = readSource(path);
  }
  return sources;
}
