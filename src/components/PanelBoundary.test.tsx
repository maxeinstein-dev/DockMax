import { render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import PanelBoundary from "./PanelBoundary";

const failure = new Error("panel exploded");

function Thrower({ failOn, panel }: { failOn: string; panel: string }) {
  if (panel === failOn) throw failure;
  return <p>Painel {panel}</p>;
}

const ignore = () => {};

function Screen({
  panel,
  onError = ignore,
}: {
  panel: string;
  onError?: (error: unknown) => void;
}) {
  return (
    <>
      <p>Dock</p>
      <PanelBoundary
        resetKey={panel}
        fallback="Falha no painel."
        onError={onError}
      >
        <Thrower failOn="settings" panel={panel} />
      </PanelBoundary>
    </>
  );
}

describe("PanelBoundary", () => {
  beforeEach(() => {
    // React reports caught render errors on console.error; expected here.
    vi.spyOn(console, "error").mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("renders its children when nothing fails", () => {
    render(
      <PanelBoundary
        resetKey="about"
        fallback="Falha no painel."
        onError={ignore}
      >
        <p>Conteúdo</p>
      </PanelBoundary>,
    );

    expect(screen.getByText("Conteúdo")).toBeInTheDocument();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("shows the fallback as an alert and reports the error once", () => {
    const onError = vi.fn();

    render(<Screen panel="settings" onError={onError} />);

    expect(screen.getByRole("alert")).toHaveTextContent("Falha no painel.");
    expect(onError).toHaveBeenCalledTimes(1);
    expect(onError).toHaveBeenCalledWith(failure);
  });

  it("keeps the rest of the app rendering when a panel fails", () => {
    render(<Screen panel="settings" />);

    expect(screen.getByText("Dock")).toBeInTheDocument();
    expect(screen.getByRole("alert")).toBeInTheDocument();
  });

  it("recovers when the reset key changes", () => {
    const { rerender } = render(<Screen panel="settings" />);
    expect(screen.getByRole("alert")).toBeInTheDocument();

    rerender(<Screen panel="about" />);

    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(screen.getByText("Painel about")).toBeInTheDocument();
  });

  it("stays failed while the reset key is the same", () => {
    let broken = true;
    function Flaky() {
      if (broken) throw failure;
      return <p>Recuperado</p>;
    }
    const tree = () => (
      <PanelBoundary
        resetKey="settings"
        fallback="Falha no painel."
        onError={ignore}
      >
        <Flaky />
      </PanelBoundary>
    );
    const { rerender } = render(tree());
    expect(screen.getByRole("alert")).toBeInTheDocument();

    broken = false;
    rerender(tree());

    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(screen.queryByText("Recuperado")).not.toBeInTheDocument();
  });
});
