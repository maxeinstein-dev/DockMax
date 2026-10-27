import { useState, type CSSProperties } from "react";
import {
  MAX_WORKSPACES,
  WORKSPACE_COLORS,
  createWorkspace,
  deleteWorkspace,
  isValidWorkspaceName,
  isWorkspaceCommandError,
  moveWorkspace,
  recolorWorkspace,
  renameWorkspace,
  type Workspace,
  type WorkspaceErrorCode,
  type WorkspacesSnapshot,
} from "../core/workspaces";
import type { MoveDirection } from "../core/widgets";
import { format, useT } from "../i18n";
import PanelFooter from "./PanelFooter";
import IconButton from "./parts/IconButton";
import InlineConfirm from "./parts/InlineConfirm";
import TextField from "./parts/TextField";
import "./Workspaces.css";

export interface WorkspacesPanelProps {
  /** The workspaces (`useWorkspaces`); null while loading, and for good when
   * they can't be read. */
  snapshot: WorkspacesSnapshot | null;
  /** The workspace being edited, whichever the docks show. */
  selected: Workspace | null;
  onSelect: (id: string) => void;
  /** Opens the programs or the widgets of the workspace being edited. */
  onEditApps: () => void;
  onEditItems: () => void;
}

/** How many programs and widgets a workspace shows. */
function counts(workspace: Workspace): { apps: number; widgets: number } {
  return {
    apps: workspace.items.filter((key) => key.startsWith("app:")).length,
    widgets: workspace.items.filter((key) => key.startsWith("widget:")).length,
  };
}

/** Configurações › Espaços de trabalho (spec 0013): every workspace, and the
 * one chosen to edit: its name, its color, its place, its programs and
 * widgets, and deleting it. Editing never changes what a dock shows. */
function WorkspacesPanel({
  snapshot,
  selected,
  onSelect,
  onEditApps,
  onEditItems,
}: WorkspacesPanelProps) {
  const t = useT();
  const text = t.workspaces;
  const [error, setError] = useState<WorkspaceErrorCode | null>(null);
  const [draft, setDraft] = useState<{ id: string; name: string } | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [busy, setBusy] = useState(false);

  const workspaces = snapshot?.workspaces ?? [];
  const name = draft !== null && draft.id === selected?.id ? draft.name : null;
  const shownName = name ?? selected?.name ?? "";
  const nameInvalid = name !== null && !isValidWorkspaceName(name);

  function fail(reason: unknown) {
    setError(isWorkspaceCommandError(reason) ? reason.code : "notReady");
  }

  function create() {
    setError(null);
    createWorkspace().then((made) => onSelect(made.id), fail);
  }

  function commitName() {
    if (!selected || name === null) return;
    if (name === selected.name || nameInvalid) {
      if (name === selected.name) setDraft(null);
      return;
    }
    setError(null);
    renameWorkspace(selected.id, name).then(() => setDraft(null), fail);
  }

  function recolor(color: string) {
    if (!selected || selected.color === color) return;
    setError(null);
    recolorWorkspace(selected.id, color).catch(fail);
  }

  function move(direction: MoveDirection) {
    if (!selected) return;
    setError(null);
    moveWorkspace(selected.id, direction).catch(fail);
  }

  function remove() {
    if (!selected) return;
    setError(null);
    setBusy(true);
    const gone = selected.id;
    deleteWorkspace(gone).then(
      () => {
        setBusy(false);
        setConfirming(false);
        const next = workspaces.find((each) => each.id !== gone);
        if (next) onSelect(next.id);
      },
      (reason: unknown) => {
        setBusy(false);
        setConfirming(false);
        fail(reason);
      },
    );
  }

  const index = selected
    ? workspaces.findIndex((each) => each.id === selected.id)
    : -1;

  return (
    <div className="workspaces">
      <h1 id="panel-title">{text.title}</h1>
      <p className="workspaces-lead">{text.lead}</p>
      {snapshot === null ? (
        <p className="workspaces-lead">{text.loading}</p>
      ) : (
        <>
          <div
            className="workspace-chips"
            role="group"
            aria-label={text.listLabel}
          >
            {workspaces.map((workspace, at) => (
              <button
                key={workspace.id}
                type="button"
                className="workspace-chip"
                aria-pressed={workspace.id === selected?.id}
                onClick={() => {
                  setConfirming(false);
                  setDraft(null);
                  onSelect(workspace.id);
                }}
              >
                <span
                  className="workspace-dot"
                  style={{ "--space": workspace.color } as CSSProperties}
                  aria-hidden="true"
                />
                {workspace.name}
                <span className="workspace-shortcut" aria-hidden="true">
                  {format(text.shortcut, { n: at + 1 })}
                </span>
              </button>
            ))}
            {workspaces.length < MAX_WORKSPACES ? (
              <button
                type="button"
                className="workspace-chip add"
                onClick={create}
              >
                {text.create}
              </button>
            ) : null}
          </div>
          {selected ? (
            <div className="workspace-card">
              <TextField
                label={text.name}
                value={shownName}
                onChange={(value) => setDraft({ id: selected.id, name: value })}
                onCommit={commitName}
                invalid={nameInvalid}
                describedBy={nameInvalid ? "workspace-name-error" : undefined}
              />
              {nameInvalid ? (
                <p id="workspace-name-error" className="workspaces-error">
                  {text.errors.invalidName}
                </p>
              ) : null}
              <div
                className="workspace-colors"
                role="radiogroup"
                aria-label={text.color}
              >
                {WORKSPACE_COLORS.map((color) => (
                  <button
                    key={color}
                    type="button"
                    role="radio"
                    className="workspace-color"
                    aria-checked={
                      selected.color.toUpperCase() === color.toUpperCase()
                    }
                    aria-label={text.colors[color]}
                    style={{ "--space": color } as CSSProperties}
                    onClick={() => recolor(color)}
                  />
                ))}
              </div>
              <p className="workspaces-lead">
                {format(text.counts, counts(selected))}
              </p>
              <div className="workspace-actions">
                <button
                  type="button"
                  className="secondary-button"
                  onClick={onEditApps}
                >
                  {text.editApps}
                </button>
                <button
                  type="button"
                  className="secondary-button"
                  onClick={onEditItems}
                >
                  {text.editItems}
                </button>
                <IconButton
                  icon="up"
                  label={format(text.moveUp, { name: selected.name })}
                  disabled={index <= 0}
                  onClick={() => move("up")}
                />
                <IconButton
                  icon="down"
                  label={format(text.moveDown, { name: selected.name })}
                  disabled={index < 0 || index >= workspaces.length - 1}
                  onClick={() => move("down")}
                />
                <IconButton
                  icon="trash"
                  tone="danger"
                  label={format(text.remove, { name: selected.name })}
                  disabled={workspaces.length <= 1}
                  onClick={() => setConfirming(true)}
                />
              </div>
              {confirming ? (
                <InlineConfirm
                  title={format(text.confirmTitle, { name: selected.name })}
                  detail={text.confirmDetail}
                  cancelLabel={text.cancel}
                  confirmLabel={text.confirm}
                  onCancel={() => setConfirming(false)}
                  onConfirm={remove}
                  busy={busy}
                />
              ) : null}
            </div>
          ) : null}
        </>
      )}
      <PanelFooter alert={error && text.errors[error]}>
        {t.settings.footer}
      </PanelFooter>
    </div>
  );
}

export default WorkspacesPanel;
