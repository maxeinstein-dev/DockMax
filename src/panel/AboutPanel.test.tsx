import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { version } from "../../package.json";
import { ptBR } from "../i18n/pt-BR";
import AboutPanel from "./AboutPanel";

/** The value next to a label of the key/value list. */
function value(label: string) {
  return screen.getByText(label).nextElementSibling;
}

describe("AboutPanel", () => {
  it("shows the app name and description", () => {
    render(<AboutPanel />);

    expect(
      screen.getByRole("heading", { name: "Dockyard" }),
    ).toBeInTheDocument();
    expect(screen.getByText(ptBR.about.description)).toBeInTheDocument();
  });

  it("shows the version of package.json", () => {
    render(<AboutPanel />);

    expect(version).toMatch(/^\d+\.\d+\.\d+/);
    expect(value("Versão")).toHaveTextContent(version);
  });

  it("shows the personal-use license", () => {
    render(<AboutPanel />);

    expect(value("Licença")).toHaveTextContent("Uso pessoal");
  });

  it("shows the repository as selectable text, not a link", () => {
    render(<AboutPanel />);

    const code = value("Código");
    expect(code).toHaveTextContent("github.com/maxeinstein-dev/Dockyard");
    expect(code).toHaveClass("about-code");
    // The panel can't open the browser (no opener plugin): no link.
    expect(screen.queryByRole("link")).not.toBeInTheDocument();
  });

  it("has the footer with the close hint", () => {
    render(<AboutPanel />);

    expect(screen.getByText("Projeto independente")).toBeInTheDocument();
    expect(screen.getByText("Esc").tagName).toBe("KBD");
    expect(screen.getByText(/fecha/)).toBeInTheDocument();
  });
});
