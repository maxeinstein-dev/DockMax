import { useId, useState } from "react";
import {
  MAX_KEYWORDS,
  keywordProblems,
  type KeywordProblems,
  type WebKeyword,
} from "../core/settings";
import { format, useT } from "../i18n";
import TextField from "./parts/TextField";
import "./parts/parts.css";

type WebKeywordsFieldProps = {
  /** The keywords in use (`Settings.launcherKeywords`), the default first. */
  keywords: readonly WebKeyword[];
  /** Saves the new list. */
  onChange: (keywords: WebKeyword[]) => void;
  disabled?: boolean;
};

/** What is being edited: a new keyword, or the one at `index`. */
type Editing = { index: number | null; draft: WebKeyword };

const EMPTY: WebKeyword = { keyword: "", name: "", template: "" };

/**
 * The keywords of the launcher's web search (spec 0012 L2), in
 * Configurações › Geral: one row per keyword with Editar, Remover and, past
 * the first, Tornar padrão; a small form adds or edits one. The form says
 * what is wrong field by field and saves only what Rust would keep.
 */
function WebKeywordsField({
  keywords,
  onChange,
  disabled = false,
}: WebKeywordsFieldProps) {
  const t = useT();
  const text = t.settings.launcher.keywords;
  const titleId = useId();
  const [editing, setEditing] = useState<Editing | null>(null);
  const [tried, setTried] = useState(false);
  const full = keywords.length >= MAX_KEYWORDS;

  const others = (index: number | null) =>
    keywords.filter((_, at) => at !== index);
  const problems: KeywordProblems = editing
    ? keywordProblems(editing.draft, others(editing.index))
    : {};

  function start(index: number | null) {
    setTried(false);
    setEditing({
      index,
      draft: index === null ? EMPTY : { ...(keywords[index] ?? EMPTY) },
    });
  }

  function save() {
    if (!editing) return;
    setTried(true);
    if (Object.keys(problems).length > 0) return;
    const draft = {
      keyword: editing.draft.keyword,
      name: editing.draft.name.trim(),
      template: editing.draft.template,
    };
    const next =
      editing.index === null
        ? [...keywords, draft]
        : keywords.map((keyword, at) =>
            at === editing.index ? draft : keyword,
          );
    onChange(next);
    setEditing(null);
  }

  function edit(field: keyof WebKeyword, value: string) {
    setEditing((current) =>
      current
        ? { ...current, draft: { ...current.draft, [field]: value } }
        : current,
    );
  }

  return (
    <section className="web-keywords" aria-labelledby={titleId}>
      <div className="toggle-text">
        <span id={titleId} className="toggle-label">
          {text.label}
        </span>
        <span className="toggle-hint">{text.hint}</span>
      </div>
      {keywords.length === 0 ? (
        <p className="toggle-hint">{text.empty}</p>
      ) : (
        <ul className="web-keyword-list" aria-label={text.list}>
          {keywords.map((keyword, index) => (
            <li key={keyword.keyword} className="web-keyword">
              <div className="toggle-text">
                <span className="toggle-label">
                  <code>{keyword.keyword}</code> · {keyword.name}
                  {index === 0 ? (
                    <span className="web-keyword-default">
                      {" "}
                      · {text.isDefault}
                    </span>
                  ) : null}
                </span>
                <span className="toggle-hint web-keyword-template">
                  {keyword.template}
                </span>
              </div>
              <div className="web-keyword-actions">
                {index > 0 ? (
                  <button
                    type="button"
                    className="quiet-button"
                    disabled={disabled}
                    aria-label={`${text.makeDefault}: ${keyword.name}`}
                    onClick={() =>
                      onChange([
                        keyword,
                        ...keywords.filter((_, at) => at !== index),
                      ])
                    }
                  >
                    {text.makeDefault}
                  </button>
                ) : null}
                <button
                  type="button"
                  className="quiet-button"
                  disabled={disabled}
                  aria-label={`${text.edit}: ${keyword.name}`}
                  onClick={() => start(index)}
                >
                  {text.edit}
                </button>
                <button
                  type="button"
                  className="quiet-button"
                  disabled={disabled}
                  aria-label={`${text.remove}: ${keyword.name}`}
                  onClick={() => {
                    if (editing?.index === index) setEditing(null);
                    onChange(keywords.filter((_, at) => at !== index));
                  }}
                >
                  {text.remove}
                </button>
              </div>
            </li>
          ))}
        </ul>
      )}
      {editing ? (
        <KeywordForm
          draft={editing.draft}
          problems={tried ? problems : {}}
          onEdit={edit}
          onSave={save}
          onCancel={() => setEditing(null)}
        />
      ) : (
        <div className="web-keyword-add">
          <button
            type="button"
            className="secondary-button"
            disabled={disabled || full}
            onClick={() => start(null)}
          >
            {text.add}
          </button>
          {full ? (
            <span className="toggle-hint">
              {format(text.full, { max: MAX_KEYWORDS })}
            </span>
          ) : null}
        </div>
      )}
    </section>
  );
}

type KeywordFormProps = {
  draft: WebKeyword;
  /** The problems to show (after the first try to save). */
  problems: KeywordProblems;
  onEdit: (field: keyof WebKeyword, value: string) => void;
  onSave: () => void;
  onCancel: () => void;
};

function KeywordForm({
  draft,
  problems,
  onEdit,
  onSave,
  onCancel,
}: KeywordFormProps) {
  const t = useT();
  const text = t.settings.launcher.keywords;
  const ids = { keyword: useId(), name: useId(), template: useId() };
  const fields: {
    field: keyof WebKeyword;
    label: string;
    placeholder?: string;
  }[] = [
    { field: "keyword", label: text.keyword, placeholder: "g" },
    { field: "name", label: text.name, placeholder: "Google" },
    {
      field: "template",
      label: text.template,
      placeholder: text.templatePlaceholder,
    },
  ];
  return (
    <form
      className="web-keyword-form"
      onSubmit={(event) => {
        event.preventDefault();
        onSave();
      }}
    >
      {fields.map(({ field, label, placeholder }) => {
        const problem = problems[field];
        return (
          <div key={field}>
            <TextField
              label={label}
              value={draft[field]}
              placeholder={placeholder}
              invalid={problem !== undefined}
              describedBy={problem ? ids[field] : undefined}
              onChange={(value) => onEdit(field, value)}
            />
            {problem ? (
              <p id={ids[field]} className="field-error">
                {text.problems[problem]}
              </p>
            ) : null}
          </div>
        );
      })}
      <div className="inline-confirm-actions">
        <button type="submit" className="secondary-button">
          {text.save}
        </button>
        <button type="button" className="quiet-button" onClick={onCancel}>
          {text.cancel}
        </button>
      </div>
    </form>
  );
}

export default WebKeywordsField;
