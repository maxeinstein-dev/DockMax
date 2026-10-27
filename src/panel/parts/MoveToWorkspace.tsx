import type { WorkspacesSnapshot } from "../../core/workspaces";
import { format, useT } from "../../i18n";

/** Props of the "Mover para <espaço>" entries of a program's menu. */
export interface MoveToWorkspaceProps {
  /** The workspaces (`useWorkspaces`), or null while they can't be read. */
  workspaces: WorkspacesSnapshot | null;
  /** A workspace was chosen, by its id. */
  onMove: (workspace: string) => void;
}

/**
 * One entry for each workspace but the one in use (spec 0013 C3): the
 * program's windows there go to it and leave the screen. Nothing with a
 * single workspace.
 */
function MoveToWorkspace({ workspaces, onMove }: MoveToWorkspaceProps) {
  const t = useT();
  if (workspaces === null || workspaces.workspaces.length < 2) return null;
  const others = workspaces.workspaces.filter(
    (each) => each.id !== workspaces.inUse,
  );
  return (
    <div className="app-menu-section" role="group">
      {others.map((each) => (
        <button
          key={each.id}
          type="button"
          role="menuitem"
          className="app-menu-item"
          onClick={() => onMove(each.id)}
        >
          {format(t.apps.menu.moveTo, { name: each.name })}
        </button>
      ))}
    </div>
  );
}

export default MoveToWorkspace;
