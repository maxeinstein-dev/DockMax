import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";
import tokens from "../../themes/tokens.css?raw";
import css from "./parts.css?raw";
import { PANEL_TITLE_ID } from "./focus";
import IconButton from "./IconButton";
import InlineConfirm from "./InlineConfirm";
import InlineNotice from "./InlineNotice";
import NavRow from "./NavRow";
import PanelHeader from "./PanelHeader";
import RemoveButton from "./RemoveButton";
import SelectField from "./SelectField";
import TextField from "./TextField";

describe("PanelHeader", () => {
  it("shows the title as the panel's heading", () => {
    render(<PanelHeader title="Relógio" />);

    const heading = screen.getByRole("heading", { name: "Relógio" });
    expect(heading).toHaveAttribute("id", PANEL_TITLE_ID);
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });

  it("has the Ajustes button in a main view", () => {
    const onAdjust = vi.fn();
    render(<PanelHeader title="Nota" onAdjust={onAdjust} />);

    fireEvent.click(screen.getByRole("button", { name: "Ajustes" }));
    expect(onAdjust).toHaveBeenCalledTimes(1);
    expect(
      screen.queryByRole("button", { name: "Voltar" }),
    ).not.toBeInTheDocument();
  });

  it("has the Voltar button before the title in a subview", () => {
    const onBack = vi.fn();
    render(<PanelHeader title="Ajustes da nota" onBack={onBack} />);

    const back = screen.getByRole("button", { name: "Voltar" });
    const heading = screen.getByRole("heading", { name: "Ajustes da nota" });
    expect(
      back.compareDocumentPosition(heading) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    fireEvent.click(back);
    expect(onBack).toHaveBeenCalledTimes(1);
    expect(
      screen.queryByRole("button", { name: "Ajustes" }),
    ).not.toBeInTheDocument();
  });

  it("focuses its action only when asked, when it mounts", () => {
    const { unmount } = render(
      <PanelHeader title="Ajustes" onBack={() => {}} focusAction />,
    );
    expect(screen.getByRole("button", { name: "Voltar" })).toHaveFocus();
    unmount();

    render(<PanelHeader title="Relógio" onAdjust={() => {}} />);
    expect(screen.getByRole("button", { name: "Ajustes" })).not.toHaveFocus();
  });
});

describe("TextField", () => {
  function Controlled({ onCommit }: { onCommit: () => void }) {
    const [value, setValue] = useState("");
    return (
      <TextField
        label="Rótulo"
        placeholder="Relógio"
        value={value}
        onChange={setValue}
        onCommit={onCommit}
      />
    );
  }

  it("labels the field and reports each edit", () => {
    const onCommit = vi.fn();
    render(<Controlled onCommit={onCommit} />);

    const input = screen.getByRole("textbox", { name: "Rótulo" });
    expect(input).toHaveAttribute("placeholder", "Relógio");
    fireEvent.change(input, { target: { value: "Lisboa" } });
    expect(input).toHaveValue("Lisboa");
    expect(onCommit).not.toHaveBeenCalled();
  });

  it("commits on Enter and on blur", () => {
    const onCommit = vi.fn();
    render(<Controlled onCommit={onCommit} />);
    const input = screen.getByRole("textbox", { name: "Rótulo" });

    fireEvent.keyDown(input, { key: "a" });
    expect(onCommit).not.toHaveBeenCalled();
    fireEvent.keyDown(input, { key: "Enter" });
    fireEvent.blur(input);
    expect(onCommit).toHaveBeenCalledTimes(2);
  });

  it("has no maxLength: the limit counts code points, not UTF-16 units", () => {
    render(<TextField label="Rótulo" value="" onChange={() => {}} />);
    expect(screen.getByRole("textbox")).not.toHaveAttribute("maxlength");
  });

  it("names a field without a visible label, and marks it invalid", () => {
    render(
      <>
        <TextField
          label="Início do expediente"
          hideLabel
          type="time"
          value="09:00"
          invalid
          describedBy="why"
          onChange={() => {}}
        />
        <p id="why">Termina antes de começar.</p>
      </>,
    );

    const input = screen.getByLabelText("Início do expediente");
    expect(input).toHaveAttribute("type", "time");
    expect(input).toBeInvalid();
    expect(input).toHaveAccessibleDescription("Termina antes de começar.");
    expect(screen.queryByText("Início do expediente")).not.toBeInTheDocument();
  });
});

describe("SelectField", () => {
  const options = [
    { value: "", label: "Do sistema (São Paulo)" },
    { value: "Europe/Lisbon", label: "Lisboa · Europe/Lisbon" },
  ];

  it("labels the list and reports a different choice", () => {
    const onChange = vi.fn();
    render(
      <SelectField
        label="Fuso horário"
        value=""
        options={options}
        onChange={onChange}
      />,
    );

    const select = screen.getByRole("combobox", { name: "Fuso horário" });
    expect(select).toHaveValue("");
    fireEvent.change(select, { target: { value: "Europe/Lisbon" } });
    expect(onChange).toHaveBeenCalledWith("Europe/Lisbon");
  });
});

describe("NavRow", () => {
  it("is a button with its label, value and arrow", () => {
    const onClick = vi.fn();
    render(<NavRow label="Itens do dock" value="3 de 12" onClick={onClick} />);

    const row = screen.getByRole("button", { name: "Itens do dock 3 de 12" });
    fireEvent.click(row);
    expect(onClick).toHaveBeenCalledTimes(1);
    expect(row.querySelector(".warning-dot")).toBeNull();
  });

  it("shows the warning dot and says it to screen readers", () => {
    render(
      <NavRow
        label="Itens do dock"
        value="3 de 12"
        warning
        onClick={() => {}}
      />,
    );

    const row = screen.getByRole("button", {
      name: /^Itens do dock\s*com aviso:\s*3 de 12$/,
    });
    expect(row.querySelector(".warning-dot")).toHaveAttribute(
      "aria-hidden",
      "true",
    );
  });

  it("can take the focus when it mounts, coming back from its subview", () => {
    render(
      <NavRow label="Itens do dock" value="" focusOnMount onClick={() => {}} />,
    );
    expect(screen.getByRole("button")).toHaveFocus();
  });
});

describe("IconButton", () => {
  it("is named by its label and can be turned off", () => {
    const onClick = vi.fn();
    const { rerender } = render(
      <IconButton icon="up" label="Subir Relógio" onClick={onClick} />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Subir Relógio" }));
    expect(onClick).toHaveBeenCalledTimes(1);

    rerender(
      <IconButton icon="up" label="Subir Relógio" onClick={onClick} disabled />,
    );
    expect(
      screen.getByRole("button", { name: "Subir Relógio" }),
    ).toBeDisabled();
  });
});

describe("RemoveButton", () => {
  it("says Remover do dock", () => {
    const onClick = vi.fn();
    render(<RemoveButton onClick={onClick} />);

    fireEvent.click(screen.getByRole("button", { name: "Remover do dock" }));
    expect(onClick).toHaveBeenCalledTimes(1);
  });
});

describe("InlineConfirm", () => {
  function confirm(props: { busy?: boolean } = {}) {
    const onCancel = vi.fn();
    const onConfirm = vi.fn();
    render(
      <InlineConfirm
        title="Remover a nota apaga o texto."
        detail="Não dá para desfazer."
        cancelLabel="Cancelar"
        confirmLabel="Remover nota"
        onCancel={onCancel}
        onConfirm={onConfirm}
        {...props}
      />,
    );
    return { onCancel, onConfirm };
  }

  it("is an alert with the consequence, focused on Cancelar", () => {
    confirm();

    expect(screen.getByRole("alert")).toHaveTextContent(
      "Remover a nota apaga o texto. Não dá para desfazer.",
    );
    expect(screen.getByRole("button", { name: "Cancelar" })).toHaveFocus();
  });

  it("cancels or confirms", () => {
    const { onCancel, onConfirm } = confirm();

    fireEvent.click(screen.getByRole("button", { name: "Cancelar" }));
    fireEvent.click(screen.getByRole("button", { name: "Remover nota" }));
    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(onConfirm).toHaveBeenCalledTimes(1);
  });

  it("turns both buttons off while busy", () => {
    confirm({ busy: true });

    for (const button of screen.getAllByRole("button")) {
      expect(button).toBeDisabled();
    }
  });
});

describe("InlineNotice", () => {
  it("is a status with its text", () => {
    render(
      <InlineNotice
        title="Os widgets estão indisponíveis."
        detail="O motivo está no log do DockMax."
      />,
    );

    expect(screen.getByRole("status")).toHaveTextContent(
      "Os widgets estão indisponíveis. O motivo está no log do DockMax.",
    );
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });

  it("has an optional action", () => {
    const onClick = vi.fn();
    render(
      <InlineNotice
        title="O banco de dados estava danificado e foi recriado."
        action={{ label: "Entendi", onClick }}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Entendi" }));
    expect(onClick).toHaveBeenCalledTimes(1);
  });
});

/* Guard for every lane: the panel parts style only through tokens. The
 * strict CSP guard (no inline style, no HTML from a string) covers all of
 * src in src/core/csp.guard.test.ts. */
describe("panel part guards", () => {
  it("uses only tokens that tokens.css defines", () => {
    const defined = new Set(
      [...tokens.matchAll(/(--[\w-]+)\s*:/g)].map((match) => match[1]),
    );
    const used = [...css.matchAll(/var\((--[\w-]+)/g)].map((match) => match[1]);
    expect(used.length).toBeGreaterThan(0);
    expect(used.filter((token) => !defined.has(token))).toEqual([]);
  });
});
