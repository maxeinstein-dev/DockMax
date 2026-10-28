import { useState, type FormEvent } from "react";
import { errorKind, log } from "../../core/log";
import { MAX_TASK_CHARS, MAX_TASKS, type TodoItem } from "../../core/widgets";
import { format, plural, useT } from "../../i18n";
import PanelFooter from "../../panel/PanelFooter";
import IconButton from "../../panel/parts/IconButton";
import PanelHeader from "../../panel/parts/PanelHeader";
import type { WidgetPanelProps } from "../contract";
import { widgetName } from "../names";
import "./todo.css";

/** A new task's ID: unique in its list. */
function newId(): string {
  return crypto.randomUUID().slice(0, 13);
}

/**
 * The main view of a Lista de tarefas panel (spec 0017 F3): a field to add
 * a task, the open ones with their mark, then the done ones. Every change
 * is saved at once, in the widget's config; the dock shows how many are
 * open.
 */
function TodoPanel({
  instance,
  onAdjust,
  returnedFromSettings,
  onUpdate,
}: WidgetPanelProps<"todo">) {
  const t = useT();
  const text = t.widgets.todo;
  // What the panel shows: the saved list, or the change on its way.
  const [shown, setShown] = useState(instance.config.items);
  const [saved, setSaved] = useState(instance.config.items);
  if (saved !== instance.config.items) {
    setSaved(instance.config.items);
    setShown(instance.config.items);
  }
  const [draft, setDraft] = useState("");
  const [failed, setFailed] = useState(false);

  function save(items: TodoItem[]) {
    setShown(items);
    setFailed(false);
    onUpdate?.({ label: instance.label, config: { items } }).catch(
      (error: unknown) => {
        log.warn(`todo: save failed: ${errorKind(error)}`);
        setShown(instance.config.items);
        setFailed(true);
      },
    );
  }

  const task = draft.trim();
  const full = shown.length >= MAX_TASKS;
  const tooLong = [...task].length > MAX_TASK_CHARS;

  function add(event: FormEvent) {
    event.preventDefault();
    if (task === "" || full || tooLong) return;
    save([...shown, { id: newId(), text: task, done: false }]);
    setDraft("");
  }

  const toggle = (id: string, done: boolean) =>
    save(shown.map((item) => (item.id === id ? { ...item, done } : item)));
  const remove = (id: string) => save(shown.filter((item) => item.id !== id));

  const open = shown.filter((item) => !item.done);
  const done = shown.filter((item) => item.done);
  const row = (item: TodoItem) => (
    <li key={item.id} className={item.done ? "todo-item done" : "todo-item"}>
      <label>
        <input
          type="checkbox"
          checked={item.done}
          onChange={(event) => toggle(item.id, event.target.checked)}
        />
        <span>{item.text}</span>
      </label>
      <IconButton
        icon="trash"
        tone="danger"
        label={format(text.remove, { text: item.text })}
        onClick={() => remove(item.id)}
      />
    </li>
  );

  return (
    <>
      <PanelHeader
        title={widgetName(t, instance)}
        onAdjust={onAdjust}
        focusAction={returnedFromSettings}
      />
      <form className="todo-add" onSubmit={add}>
        <input
          className="field-input"
          aria-label={text.new}
          placeholder={full ? text.full : text.placeholder}
          value={draft}
          disabled={full}
          aria-invalid={tooLong}
          onChange={(event) => setDraft(event.target.value)}
        />
        <button
          type="submit"
          className="secondary-button"
          disabled={task === "" || full || tooLong}
        >
          {text.add}
        </button>
      </form>
      {shown.length === 0 ? (
        <p className="todo-note">{text.empty}</p>
      ) : (
        <div className="todo-lists">
          {open.length > 0 ? (
            <ul className="todo-list" aria-label={text.openTitle}>
              {open.map(row)}
            </ul>
          ) : (
            <p className="todo-note">{text.allDone}</p>
          )}
          {done.length > 0 ? (
            <ul
              className="todo-list"
              aria-label={plural(text.doneTitle, done.length)}
            >
              {done.map(row)}
            </ul>
          ) : null}
        </div>
      )}
      <PanelFooter alert={failed ? t.widgets.errors.invalidTasks : null}>
        {plural(text.footer, open.length)}
      </PanelFooter>
    </>
  );
}

export default TodoPanel;
