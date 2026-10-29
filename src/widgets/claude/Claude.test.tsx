import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { resetText, type ClaudeReading } from "../../core/claude";
import type { WidgetInstance } from "../../core/widgets";
import { ptBR } from "../../i18n/pt-BR";
import ClaudePanel from "./ClaudePanel";

const text = ptBR.widgets.claude;

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const claude: WidgetInstance<"claude"> = {
  id: "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6098",
  label: null,
  kind: "claude",
  config: { width: 2 },
};

/** 29/10/2026, 11:00 local. */
const NOW = new Date(2026, 9, 29, 11, 0);
const SECONDS = Math.floor(NOW.getTime() / 1000);

function panel(reading: ClaudeReading | null) {
  invoke.mockImplementation(() => Promise.resolve(reading));
  render(
    <ClaudePanel
      instance={claude}
      now={NOW}
      onAdjust={() => undefined}
      returnedFromSettings={false}
    />,
  );
}

beforeEach(() => {
  invoke.mockReset();
});

describe("ClaudePanel", () => {
  it("shows both limits, used and when they start over", async () => {
    panel({
      state: "ready",
      at: SECONDS,
      usage: {
        fiveHour: { percent: 37.4, resetsAt: SECONDS + 2 * 3600 + 14 * 60 },
        sevenDay: { percent: 12, resetsAt: SECONDS + 3 * 24 * 3600 },
      },
    });
    expect(await screen.findByText("37% usado")).toBeInTheDocument();
    expect(screen.getByText("Reinicia em 2 h 14 min")).toBeInTheDocument();
    expect(screen.getByText("12% usado")).toBeInTheDocument();
    expect(screen.getByText(text.fiveHour)).toBeInTheDocument();
    expect(screen.getByText(text.week)).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith("claude_usage", { fresh: true });
  });

  it("says when the Claude Code is not signed in, or expired", async () => {
    panel({ state: "signedOut" });
    expect(await screen.findByText(text.states.signedOut)).toBeInTheDocument();
  });

  it("opens the Claude Code's sign-in", async () => {
    panel({ state: "expired" });
    fireEvent.click(await screen.findByRole("button", { name: text.signIn }));
    expect(await screen.findByText(text.signInOpened)).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith("claude_sign_in");
  });
});

describe("resetText", () => {
  const words = text.resetWords;
  it("counts hours and minutes within a day", () => {
    expect(resetText(SECONDS + 45 * 60, SECONDS, words)).toBe("em 45 min");
    expect(resetText(SECONDS + 3 * 3600, SECONDS, words)).toBe("em 3 h");
    expect(resetText(SECONDS + 30, SECONDS, words)).toBe("em instantes");
  });

  it("names the weekday and hour beyond a day", () => {
    const later = resetText(SECONDS + 3 * 24 * 3600, SECONDS, words);
    expect(later).toMatch(/\d{2}:\d{2}/);
  });
});
