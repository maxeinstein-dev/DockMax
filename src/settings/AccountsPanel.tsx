import { useLayoutEffect, useRef, useState } from "react";
import {
  connectGoogleAccount,
  disconnectGoogleAccount,
  googleCalendars,
  setGoogleClient,
  useGoogleStatus,
  type GoogleCalendar,
  type GoogleErrorCode,
} from "../core/google";
import { format, useT } from "../i18n";
import InlineConfirm from "../panel/parts/InlineConfirm";
import SettingsGroup from "../panel/parts/SettingsGroup";
import TextField from "../panel/parts/TextField";
import "./AccountsPanel.css";

function codeOf(reason: unknown): GoogleErrorCode {
  return typeof reason === "string" ? (reason as GoogleErrorCode) : "notReady";
}

/** A calendar's color dot. The color goes through the CSSOM, never a
 * `style` attribute (strict CSP). Decorative: the name is next to it. */
function Swatch({ color }: { color: string | null }) {
  const dot = useRef<HTMLSpanElement>(null);
  useLayoutEffect(() => {
    if (color && /^#[0-9a-f]{6}$/i.test(color)) {
      dot.current?.style.setProperty("--swatch", color);
    }
  }, [color]);
  return <span ref={dot} className="account-swatch" aria-hidden="true" />;
}

/** The OAuth client: the fields while none is set (or while changing it),
 * its id otherwise. The secret is sent once and never shown again. */
function ClientGroup({
  clientId,
  onSaved,
}: {
  clientId: string | null;
  onSaved: () => void;
}) {
  const t = useT();
  const text = t.accounts.client;
  const [editing, setEditing] = useState(false);
  const [id, setId] = useState("");
  const [secret, setSecret] = useState("");
  const [error, setError] = useState<GoogleErrorCode | null>(null);
  const [saving, setSaving] = useState(false);

  function save() {
    setError(null);
    setSaving(true);
    setGoogleClient(id, secret).then(
      () => {
        setSaving(false);
        setEditing(false);
        setId("");
        setSecret("");
        onSaved();
      },
      (reason: unknown) => {
        setSaving(false);
        setError(codeOf(reason));
      },
    );
  }

  const showFields = clientId === null || editing;
  return (
    <SettingsGroup title={text.title} grouped>
      {showFields ? (
        <div className="account-client">
          {clientId === null ? (
            <p className="account-text">{text.intro}</p>
          ) : null}
          <TextField
            label={text.id}
            value={id}
            invalid={error === "invalidClientId"}
            onChange={setId}
          />
          <TextField
            label={text.secret}
            type="password"
            value={secret}
            invalid={error === "invalidClientSecret"}
            onChange={setSecret}
          />
          <p className="account-hint">{text.hint}</p>
          <div className="account-actions">
            {clientId !== null ? (
              <button
                type="button"
                className="quiet-button"
                onClick={() => {
                  setEditing(false);
                  setError(null);
                }}
              >
                {text.cancel}
              </button>
            ) : null}
            <button
              type="button"
              className="secondary-button"
              disabled={saving || id.trim() === "" || secret.trim() === ""}
              onClick={save}
            >
              {text.save}
            </button>
          </div>
        </div>
      ) : (
        <div className="account-row">
          <span className="account-text account-id">
            {format(text.current, { id: clientId })}
          </span>
          <button
            type="button"
            className="quiet-button"
            onClick={() => setEditing(true)}
          >
            {text.change}
          </button>
        </div>
      )}
      {error ? (
        <p className="account-error" role="alert">
          {t.accounts.errors[error]}
        </p>
      ) : null}
    </SettingsGroup>
  );
}

/** One account: its e-mail, its calendars on demand, and Desconectar,
 * which asks first. */
function AccountRow({
  email,
  onError,
}: {
  email: string;
  onError: (code: GoogleErrorCode) => void;
}) {
  const t = useT();
  const text = t.accounts.list;
  const [calendars, setCalendars] = useState<GoogleCalendar[] | null>(null);
  const [open, setOpen] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const [busy, setBusy] = useState(false);

  function toggle() {
    const next = !open;
    setOpen(next);
    if (next && calendars === null) {
      googleCalendars(email).then(setCalendars, (reason: unknown) => {
        setOpen(false);
        onError(codeOf(reason));
      });
    }
  }

  function disconnect() {
    setBusy(true);
    disconnectGoogleAccount(email).then(
      () => setBusy(false),
      (reason: unknown) => {
        setBusy(false);
        setConfirming(false);
        onError(codeOf(reason));
      },
    );
  }

  return (
    <li className="account">
      <div className="account-row">
        <span className="account-email">{email}</span>
        <button
          type="button"
          className="quiet-button"
          aria-expanded={open}
          aria-label={format(open ? text.hideCalendars : text.showCalendars, {
            email,
          })}
          onClick={toggle}
        >
          {text.calendars}
        </button>
        {confirming ? null : (
          <button
            type="button"
            className="quiet-button"
            aria-label={format(text.disconnectNamed, { email })}
            onClick={() => setConfirming(true)}
          >
            {text.disconnect}
          </button>
        )}
      </div>
      {confirming ? (
        <InlineConfirm
          title={text.confirmDisconnect.title}
          detail={text.confirmDisconnect.detail}
          cancelLabel={text.confirmDisconnect.cancel}
          confirmLabel={text.confirmDisconnect.confirm}
          onCancel={() => setConfirming(false)}
          onConfirm={disconnect}
          busy={busy}
        />
      ) : null}
      {open ? (
        calendars === null ? (
          <p className="account-text">{text.loading}</p>
        ) : calendars.length === 0 ? (
          <p className="account-text">{text.noCalendars}</p>
        ) : (
          <ul className="account-calendars">
            {calendars.map((calendar) => (
              <li key={calendar.id}>
                <Swatch color={calendar.color} />
                {calendar.name}
                {calendar.primary ? (
                  <span className="account-tag"> · {text.primary}</span>
                ) : null}
              </li>
            ))}
          </ul>
        )
      ) : null}
    </li>
  );
}

/**
 * Configurações › Contas (spec 0017 F2a, ADR 0012): the Google app the
 * author made, and the accounts connected through the browser, with their
 * calendars. The tokens stay in the Windows vault.
 */
function AccountsPanel() {
  const t = useT();
  const text = t.accounts.list;
  const { status, refresh } = useGoogleStatus();
  const [connecting, setConnecting] = useState(false);
  const [error, setError] = useState<GoogleErrorCode | null>(null);
  const [said, setSaid] = useState("");

  if (status === null) return null;

  function connect() {
    setError(null);
    setSaid("");
    setConnecting(true);
    connectGoogleAccount().then(
      (email) => {
        setConnecting(false);
        setSaid(format(text.connected, { email }));
        refresh();
      },
      (reason: unknown) => {
        setConnecting(false);
        setError(codeOf(reason));
      },
    );
  }

  return (
    <>
      <ClientGroup clientId={status.clientId} onSaved={refresh} />
      <SettingsGroup title={text.title} grouped>
        {status.accounts.length === 0 ? (
          <p className="account-text">{text.empty}</p>
        ) : (
          <ul className="account-list">
            {status.accounts.map((email) => (
              <AccountRow key={email} email={email} onError={setError} />
            ))}
          </ul>
        )}
        <div className="account-actions">
          <button
            type="button"
            className="secondary-button"
            disabled={status.clientId === null || connecting}
            onClick={connect}
          >
            {text.connect}
          </button>
        </div>
        <p className="account-text" role="status">
          {connecting
            ? text.waiting
            : status.clientId === null
              ? text.needsClient
              : said}
        </p>
        {error ? (
          <p className="account-error" role="alert">
            {t.accounts.errors[error]}
          </p>
        ) : null}
      </SettingsGroup>
    </>
  );
}

export default AccountsPanel;
