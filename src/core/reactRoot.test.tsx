import { act, type ReactNode } from "react";
import { flushSync } from "react-dom";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import PanelBoundary from "../components/PanelBoundary";
import { ROOT_OPTIONS } from "./reactRoot";

const { log } = vi.hoisted(() => ({
  log: { error: vi.fn(), warn: vi.fn(), info: vi.fn() },
}));
vi.mock("./log", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./log")>()),
  log,
}));

function Thrower(): never {
  // The message carries a path that must never reach the console or the log.
  throw new TypeError("C:\\Users\\someone\\settings.json");
}

/** Lets `act` flush the renders of a root made here, outside Testing
 * Library. */
const actEnvironment = globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean };
const previousActEnvironment = actEnvironment.IS_REACT_ACT_ENVIRONMENT;

describe("ROOT_OPTIONS", () => {
  let container: HTMLElement;
  let root: Root | null;

  beforeEach(() => {
    for (const write of Object.values(log)) write.mockReset();
    actEnvironment.IS_REACT_ACT_ENVIRONMENT = true;
    container = document.createElement("div");
    document.body.append(container);
    root = null;
  });

  afterEach(() => {
    act(() => root?.unmount());
    container.remove();
    actEnvironment.IS_REACT_ACT_ENVIRONMENT = previousActEnvironment;
    vi.restoreAllMocks();
  });

  function consoleSpies() {
    return {
      error: vi.spyOn(globalThis.console, "error").mockImplementation(() => {}),
      warn: vi.spyOn(globalThis.console, "warn").mockImplementation(() => {}),
    };
  }

  function renderWithOptions(element: ReactNode) {
    root = createRoot(container, ROOT_OPTIONS);
    act(() => root?.render(element));
  }

  it("keeps an error caught by a boundary off the console", () => {
    const spies = consoleSpies();
    const onError = vi.fn();

    renderWithOptions(
      <PanelBoundary resetKey="settings" fallback="Falha." onError={onError}>
        <Thrower />
      </PanelBoundary>,
    );

    expect(container).toHaveTextContent("Falha.");
    expect(spies.error).not.toHaveBeenCalled();
    expect(spies.warn).not.toHaveBeenCalled();
    // The boundary reports it, once; the root does not report it again.
    expect(onError).toHaveBeenCalledTimes(1);
    expect(log.error).not.toHaveBeenCalled();
  });

  it("logs an uncaught render error by its kind only, off the console", () => {
    const spies = consoleSpies();
    // Inside `act`, React rethrows an uncaught error into the test instead
    // of calling `onUncaughtError`, so this renders outside it.
    actEnvironment.IS_REACT_ACT_ENVIRONMENT = false;
    root = createRoot(container, ROOT_OPTIONS);

    flushSync(() => root?.render(<Thrower />));

    expect(spies.error).not.toHaveBeenCalled();
    expect(log.error).toHaveBeenCalledTimes(1);
    expect(log.error).toHaveBeenCalledWith("react: render failed: TypeError");
  });

  it("logs a recoverable error by its kind only", () => {
    ROOT_OPTIONS.onRecoverableError(new TypeError("C:\\Users\\someone"));

    expect(log.warn).toHaveBeenCalledWith(
      "react: recovered from a render error: TypeError",
    );
  });

  it("would print the caught error without them (control)", () => {
    const spies = consoleSpies();
    root = createRoot(container);

    act(() =>
      root?.render(
        <PanelBoundary resetKey="settings" fallback="Falha.">
          <Thrower />
        </PanelBoundary>,
      ),
    );

    expect(spies.error).toHaveBeenCalled();
  });
});

const createRootMock = vi.hoisted(() =>
  vi.fn(() => ({ render: vi.fn(), unmount: vi.fn() })),
);

describe.each(["dock", "panel"])("%s window entry", (window) => {
  beforeEach(() => {
    vi.resetModules();
    vi.doMock("react-dom/client", () => ({
      default: { createRoot: createRootMock },
      createRoot: createRootMock,
    }));
    createRootMock.mockClear();
    document.body.innerHTML = '<div id="root"></div>';
  });

  afterEach(() => {
    vi.doUnmock("react-dom/client");
    document.body.innerHTML = "";
  });

  it("creates its root with the error options", async () => {
    const { ROOT_OPTIONS: options } = await import("./reactRoot");
    await import(`../${window}/main.tsx`);

    expect(createRootMock).toHaveBeenCalledTimes(1);
    expect(createRootMock).toHaveBeenCalledWith(
      document.getElementById("root"),
      options,
    );
  });
});
