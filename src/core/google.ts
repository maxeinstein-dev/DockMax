import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { errorKind, log } from "./log";

/* The Google accounts (spec 0017 F2a, ADR 0012). Must match
 * `src-tauri/src/commands/google.rs`. Only Configurações calls these; the
 * client's secret goes to Rust once and never comes back. */

/** Why a call failed: only a code. */
export type GoogleErrorCode =
  | "noClient"
  | "invalidClientId"
  | "invalidClientSecret"
  | "refused"
  | "timedOut"
  /** Cancelar, or a new sign-in started: not an error to show. */
  | "cancelled"
  | "offline"
  | "signInAgain"
  | "notConnected"
  | "unexpected"
  | "vault"
  | "notAllowed"
  /** TS only: the command never ran. */
  | "notReady";

const CODES: ReadonlySet<string> = new Set<GoogleErrorCode>([
  "noClient",
  "invalidClientId",
  "invalidClientSecret",
  "refused",
  "timedOut",
  "cancelled",
  "offline",
  "signInAgain",
  "notConnected",
  "unexpected",
  "vault",
  "notAllowed",
]);

/** The client set (its id only) and the accounts, by e-mail. */
export interface GoogleStatus {
  clientId: string | null;
  accounts: string[];
}

/** A calendar of an account. */
export interface GoogleCalendar {
  id: string;
  name: string;
  /** `#rrggbb`, as chosen in Google Agenda. */
  color: string | null;
  primary: boolean;
}

/** Event every window gets when an account comes or goes. */
export const GOOGLE_ACCOUNTS_EVENT = "google-accounts-changed";

export const GOOGLE_COMMANDS = {
  status: "google_status",
  setClient: "set_google_client",
  connect: "connect_google_account",
  cancel: "cancel_google_sign_in",
  calendars: "google_calendars",
  disconnect: "disconnect_google_account",
} as const;

/** Runs `command`; any failure rejects with its code only. */
function run<T>(
  command: string,
  args: Record<string, unknown> = {},
): Promise<T> {
  return invoke<T>(command, args).catch((error: unknown) => {
    if (typeof error === "string" && CODES.has(error)) throw error;
    log.warn(`google: a command failed: ${errorKind(error)}`);
    const code: GoogleErrorCode = "notReady";
    throw code;
  });
}

export function googleStatus(): Promise<GoogleStatus> {
  return run(GOOGLE_COMMANDS.status);
}

/** Keeps the OAuth client the author made in Google Cloud. */
export function setGoogleClient(id: string, secret: string): Promise<void> {
  return run(GOOGLE_COMMANDS.setClient, { id, secret });
}

/** Opens the browser on Google's sign-in; resolves with the account's
 * e-mail once it came back (up to 5 minutes). */
export function connectGoogleAccount(): Promise<string> {
  return run(GOOGLE_COMMANDS.connect);
}

/** Ends the sign-in waiting for the browser: `connectGoogleAccount`
 * rejects with `cancelled`. */
export function cancelGoogleSignIn(): Promise<void> {
  return run(GOOGLE_COMMANDS.cancel);
}

export function googleCalendars(email: string): Promise<GoogleCalendar[]> {
  return run(GOOGLE_COMMANDS.calendars, { email });
}

export function disconnectGoogleAccount(email: string): Promise<void> {
  return run(GOOGLE_COMMANDS.disconnect, { email });
}

/** The client and the accounts, read again when an account comes or goes;
 * null while loading. `refresh` reads again (after the client changed). */
export function useGoogleStatus(): {
  status: GoogleStatus | null;
  refresh: () => void;
} {
  const [status, setStatus] = useState<GoogleStatus | null>(null);
  const refresh = useCallback(() => {
    googleStatus().then(setStatus, () => setStatus(null));
  }, []);
  useEffect(() => {
    refresh();
    const unlisten = listen(GOOGLE_ACCOUNTS_EVENT, refresh);
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, [refresh]);
  return { status, refresh };
}
