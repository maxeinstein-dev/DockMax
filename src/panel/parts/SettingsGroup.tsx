import { useId, type ReactNode } from "react";
import "./parts.css";

/**
 * A group of rows of the Configurações app (spec 0014 R1): a small title
 * and, under it, a card with the rows. In the panel (`grouped` off) the rows
 * stay as they were, one after the other.
 */
function SettingsGroup({
  title,
  grouped,
  children,
}: {
  title: string;
  grouped: boolean;
  children: ReactNode;
}) {
  const titleId = useId();
  if (!grouped) return <>{children}</>;
  return (
    <section className="settings-group" aria-labelledby={titleId}>
      <h2 id={titleId} className="settings-group-title">
        {title}
      </h2>
      <div className="settings-group-card">{children}</div>
    </section>
  );
}

export default SettingsGroup;
