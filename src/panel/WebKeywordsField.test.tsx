import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import {
  MAX_KEYWORDS,
  keywordProblems,
  type WebKeyword,
} from "../core/settings";
import { ptBR } from "../i18n/pt-BR";
import WebKeywordsField from "./WebKeywordsField";

const text = ptBR.settings.launcher.keywords;

const GOOGLE: WebKeyword = {
  keyword: "g",
  name: "Google",
  template: "https://www.google.com/search?q=%s",
};
const YOUTUBE: WebKeyword = {
  keyword: "yt",
  name: "YouTube",
  template: "https://www.youtube.com/results?search_query=%s",
};

function renderField(keywords: WebKeyword[] = [GOOGLE, YOUTUBE]) {
  const onChange = vi.fn();
  render(<WebKeywordsField keywords={keywords} onChange={onChange} />);
  return onChange;
}

const field = (name: string) => screen.getByRole("textbox", { name });
const type = (name: string, value: string) =>
  fireEvent.change(field(name), { target: { value } });

describe("WebKeywordsField", () => {
  it("lists the keywords, the first as the default", () => {
    renderField();

    const rows = within(
      screen.getByRole("list", { name: text.list }),
    ).getAllByRole("listitem");
    expect(rows).toHaveLength(2);
    expect(rows[0]).toHaveTextContent(`g · Google · ${text.isDefault}`);
    expect(rows[1]).not.toHaveTextContent(text.isDefault);
  });

  it("adds a keyword at the end", () => {
    const onChange = renderField();

    fireEvent.click(screen.getByRole("button", { name: text.add }));
    type(text.keyword, "d");
    type(text.name, " Duck ");
    type(text.template, "https://duckduckgo.com/?q=%s");
    fireEvent.click(screen.getByRole("button", { name: text.save }));

    expect(onChange).toHaveBeenCalledWith([
      GOOGLE,
      YOUTUBE,
      { keyword: "d", name: "Duck", template: "https://duckduckgo.com/?q=%s" },
    ]);
    expect(screen.queryByRole("button", { name: text.save })).toBeNull();
  });

  it("says what is wrong and saves nothing", () => {
    const onChange = renderField();

    fireEvent.click(screen.getByRole("button", { name: text.add }));
    type(text.keyword, "G");
    type(text.template, "http://example.com/?q=%s");
    fireEvent.click(screen.getByRole("button", { name: text.save }));

    expect(onChange).not.toHaveBeenCalled();
    expect(field(text.keyword)).toHaveAttribute("aria-invalid", "true");
    expect(field(text.keyword)).toHaveAccessibleDescription(
      text.problems.keywordRepeated,
    );
    expect(field(text.name)).toHaveAccessibleDescription(
      text.problems.nameEmpty,
    );
    expect(field(text.template)).toHaveAccessibleDescription(
      text.problems.templateNotHttps,
    );
  });

  it("edits a keyword in its place", () => {
    const onChange = renderField();

    fireEvent.click(
      screen.getByRole("button", { name: `${text.edit}: YouTube` }),
    );
    expect(field(text.keyword)).toHaveValue("yt");
    type(text.keyword, "y");
    fireEvent.click(screen.getByRole("button", { name: text.save }));

    expect(onChange).toHaveBeenCalledWith([
      GOOGLE,
      { ...YOUTUBE, keyword: "y" },
    ]);
  });

  it("keeps its own word when edited", () => {
    const onChange = renderField();

    fireEvent.click(
      screen.getByRole("button", { name: `${text.edit}: Google` }),
    );
    fireEvent.click(screen.getByRole("button", { name: text.save }));

    expect(onChange).toHaveBeenCalledWith([GOOGLE, YOUTUBE]);
  });

  it("removes a keyword and makes another the default", () => {
    const onChange = renderField();

    fireEvent.click(
      screen.getByRole("button", { name: `${text.makeDefault}: YouTube` }),
    );
    expect(onChange).toHaveBeenLastCalledWith([YOUTUBE, GOOGLE]);
    expect(
      screen.queryByRole("button", { name: `${text.makeDefault}: Google` }),
    ).toBeNull();

    fireEvent.click(
      screen.getByRole("button", { name: `${text.remove}: Google` }),
    );
    expect(onChange).toHaveBeenLastCalledWith([YOUTUBE]);
  });

  it("cancels without saving", () => {
    const onChange = renderField();

    fireEvent.click(screen.getByRole("button", { name: text.add }));
    type(text.keyword, "d");
    fireEvent.click(screen.getByRole("button", { name: text.cancel }));

    expect(onChange).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: text.add })).toBeEnabled();
  });

  it("adds no keyword past the most", () => {
    renderField(
      Array.from({ length: MAX_KEYWORDS }, (_, n) => ({
        ...GOOGLE,
        keyword: `k${n}`,
      })),
    );

    expect(screen.getByRole("button", { name: text.add })).toBeDisabled();
    expect(
      screen.getByText(text.full.replace("{max}", String(MAX_KEYWORDS))),
    ).toBeInTheDocument();
  });

  it("says when there are none", () => {
    renderField([]);

    expect(screen.getByText(text.empty)).toBeInTheDocument();
  });
});

describe("keywordProblems", () => {
  const ok = { keyword: "d", name: "Duck", template: "https://d.com/?q=%s" };

  it("accepts what Rust keeps", () => {
    expect(keywordProblems(ok, [GOOGLE])).toEqual({});
    expect(keywordProblems({ ...ok, keyword: "ação" }, [])).toEqual({});
  });

  it.each([
    [{ keyword: "" }, { keyword: "keywordEmpty" }],
    [{ keyword: "a b" }, { keyword: "keywordSpaces" }],
    [{ keyword: "a".repeat(17) }, { keyword: "keywordTooLong" }],
    [{ keyword: "G" }, { keyword: "keywordRepeated" }],
    [{ name: "  " }, { name: "nameEmpty" }],
    [{ name: "a".repeat(41) }, { name: "nameTooLong" }],
    [{ template: "http://d.com/?q=%s" }, { template: "templateNotHttps" }],
    [{ template: "https://?q=%s" }, { template: "templateNotHttps" }],
    [{ template: "https://%s.com/" }, { template: "templateNotHttps" }],
    [{ template: "https://d.com/?q=" }, { template: "templateNoTerm" }],
    [{ template: "https://d.com/?q=%s x" }, { template: "templateSpaces" }],
    [
      { template: `https://d.com/?q=%s${"a".repeat(500)}` },
      { template: "templateTooLong" },
    ],
  ])("refuses %o", (change, problems) => {
    expect(keywordProblems({ ...ok, ...change }, [GOOGLE])).toEqual(problems);
  });
});
