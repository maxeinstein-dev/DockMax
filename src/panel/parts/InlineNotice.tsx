import Icon from "./Icon";
import "./parts.css";

export interface InlineNoticeProps {
  /** The main sentence, in strong text. */
  title: string;
  /** What follows it, in the secondary ink. */
  detail?: string;
  /** An optional action under the text, such as "Entendi". */
  action?: { label: string; onClick: () => void; disabled?: boolean };
}

/** Inline notice (DESIGN.md "Aviso inline"): the database recovered or
 * unavailable, the clock's unknown zone. A status, so screen readers read
 * it without moving the focus. */
function InlineNotice({ title, detail, action }: InlineNoticeProps) {
  return (
    <div className="inline-notice" role="status">
      <Icon name="warn" />
      <p>
        <strong>{title}</strong>
        {detail ? ` ${detail}` : null}
      </p>
      {action ? (
        <button
          type="button"
          className="secondary-button"
          disabled={action.disabled}
          onClick={action.onClick}
        >
          {action.label}
        </button>
      ) : null}
    </div>
  );
}

export default InlineNotice;
