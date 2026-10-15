import {
  error as writeError,
  info as writeInfo,
  warn as writeWarn,
} from "@tauri-apps/plugin-log";

/* Frontend logs, written by the log plugin to the same rotating file as the
 * Rust side (spec 0002 §2.3 item 5). A message names the capability affected
 * and the kind of error only, like "panel: render failed: TypeError": never
 * settings content, a file path, user data or an error message (it may
 * carry any of them). */

type Write = (message: string) => Promise<void>;

/** Never throws: logging must not break the caller. A message the plugin
 * could not write goes to the console instead. */
function send(write: Write, message: string) {
  write(message).catch(() => {
    console.warn(message);
  });
}

export const log = {
  error: (message: string) => send(writeError, message),
  warn: (message: string) => send(writeWarn, message),
  info: (message: string) => send(writeInfo, message),
};

/** Longest error name kept; anything longer is not a class name. */
const MAX_NAME_LENGTH = 64;

/** The kind of `error`, safe to log: the name of an `Error` ("TypeError")
 * when it looks like an identifier, else "Error". Other thrown values give
 * their type ("string"), never their content. */
export function errorKind(error: unknown): string {
  if (!(error instanceof Error)) return typeof error;
  const { name } = error;
  return name.length <= MAX_NAME_LENGTH && /^[A-Za-z_$][\w$]*$/.test(name)
    ? name
    : "Error";
}
