import js from "@eslint/js";
import eslintConfigPrettier from "eslint-config-prettier/flat";
import reactHooks from "eslint-plugin-react-hooks";
import { reactRefresh } from "eslint-plugin-react-refresh";
import { defineConfig, globalIgnores } from "eslint/config";
import globals from "globals";
import tseslint from "typescript-eslint";

/** A call to the frontend log: `log.error|warn|info(...)`. */
const LOG_CALL =
  'CallExpression[callee.type="MemberExpression"][callee.object.name="log"][callee.property.name=/^(error|warn|info)$/]';

export default defineConfig(
  globalIgnores(["dist", "src-tauri", "node_modules", "docs"]),
  {
    files: ["**/*.{js,ts,tsx}"],
    extends: [
      js.configs.recommended,
      tseslint.configs.recommended,
      reactHooks.configs.flat.recommended,
    ],
    languageOptions: {
      globals: globals.browser,
    },
  },
  {
    files: ["**/*.tsx"],
    extends: [reactRefresh.configs.vite()],
  },
  {
    // Frontend logs name the capability and the kind of error only (ADR
    // 0007): a message is one string literal, or a template literal whose
    // only interpolations are `errorKind(...)`. Pinned by
    // src/core/log.lint.test.ts.
    files: ["src/**/*.{ts,tsx}"],
    rules: {
      "no-restricted-syntax": [
        "error",
        {
          selector: `${LOG_CALL}[arguments.length!=1]`,
          message: "A log call takes exactly one message.",
        },
        {
          selector: `${LOG_CALL}[arguments.length=1][arguments.0.type!=/^(Literal|TemplateLiteral)$/]`,
          message:
            "Log messages must be a string or template literal, so what they carry stays visible.",
        },
        {
          selector: `${LOG_CALL} > TemplateLiteral > :not(TemplateElement, CallExpression[callee.name="errorKind"])`,
          message:
            "Log only the kind of an error, through errorKind(...): never its message, a path, settings or other data.",
        },
        {
          selector: 'ImportSpecifier[imported.name="log"][local.name!="log"]',
          message:
            "Import `log` under its own name, so the log rules can see the calls.",
        },
      ],
      "no-restricted-imports": [
        "error",
        {
          paths: [
            {
              name: "@tauri-apps/plugin-log",
              message:
                "Log through src/core/log.ts, which keeps data out of the messages.",
            },
          ],
        },
      ],
    },
  },
  {
    // The one place allowed to talk to the log plugin.
    files: ["src/core/log.ts"],
    rules: { "no-restricted-imports": "off" },
  },
  {
    files: ["vite.config.ts", "eslint.config.js"],
    languageOptions: {
      globals: globals.node,
    },
  },
  {
    // E2E suite: runs in Node under WebdriverIO's Mocha runner; only the
    // callbacks given to `browser.execute` run in the page.
    files: ["e2e/**/*.ts"],
    languageOptions: {
      globals: { ...globals.node, ...globals.mocha },
    },
  },
  eslintConfigPrettier,
);
