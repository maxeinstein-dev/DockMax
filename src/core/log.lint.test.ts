// @vitest-environment node
import { ESLint } from "eslint";
import { describe, expect, it } from "vitest";

/* Pins the lint rule that keeps data out of the frontend logs (ADR 0007):
 * a `log.error|warn|info` call takes one string literal, or a template
 * literal whose only interpolations are `errorKind(...)`. */

const RULE = "no-restricted-syntax";

const eslint = new ESLint();

/** The messages of RULE for `code` as a component of the project. */
async function violations(code: string): Promise<string[]> {
  const source = `import { errorKind, log } from "../core/log";\n${code}\n`;
  const [result] = await eslint.lintText(source, {
    filePath: "src/dock/Example.ts",
  });
  return (result?.messages ?? [])
    .filter((message) => message.ruleId === RULE)
    .map((message) => message.message);
}

describe("frontend log rule", () => {
  it.each([
    'log.error("dock: ready");',
    "log.warn(`settings: read failed`);",
    "log.info(`panel: render failed: ${errorKind(new Error())}`);",
    "const error: unknown = 1;\nlog.error(`a: ${errorKind(error)}: ${errorKind(error)}`);",
  ])("accepts %s", async (code) => {
    expect(await violations(code)).toEqual([]);
  });

  it.each([
    "const error: unknown = 1;\nlog.error(`dock: toggle panel failed: ${String(error)}`);",
    "const error = new Error();\nlog.warn(`settings: ${error.message}`);",
    'const path = "C:";\nlog.info(`settings: ${path}`);',
    "const error: unknown = 1;\nlog.error(`dock: ${errorKind(error)} ${JSON.stringify(error)}`);",
    'const message = "x";\nlog.error(message);',
    "const error: unknown = 1;\nlog.error(String(error));",
    'log.warn("settings: " + String(1));',
    'log.info("a", "b");',
    "log.error();",
  ])("rejects %s", async (code) => {
    expect(await violations(code)).toHaveLength(1);
  });
});
