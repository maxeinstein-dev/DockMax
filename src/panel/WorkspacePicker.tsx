import type { CSSProperties } from "react";
import type { Workspace } from "../core/workspaces";
import { useT } from "../i18n";
import "./Workspaces.css";

/** Which workspace the programs or the widgets below belong to (spec 0013):
 * a chip each, the one being edited pressed. Nothing with a single
 * workspace, where there is nothing to choose. */
function WorkspacePicker({
  workspaces,
  selected,
  onSelect,
}: {
  workspaces: readonly Workspace[];
  selected: Workspace | null;
  onSelect: (id: string) => void;
}) {
  const text = useT().workspaces;
  if (workspaces.length < 2) return null;
  return (
    <div className="workspace-chips" role="group" aria-label={text.editing}>
      {workspaces.map((workspace) => (
        <button
          key={workspace.id}
          type="button"
          className="workspace-chip"
          aria-pressed={workspace.id === selected?.id}
          onClick={() => onSelect(workspace.id)}
        >
          <span
            className="workspace-dot"
            style={{ "--space": workspace.color } as CSSProperties}
            aria-hidden="true"
          />
          {workspace.name}
        </button>
      ))}
    </div>
  );
}

export default WorkspacePicker;
