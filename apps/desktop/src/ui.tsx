import { useEffect, useId, useRef, useState, type ReactNode } from 'react';
import { t } from './i18n';

/**
 * The handful of controls every page is built from.
 *
 * They exist so a toggle looks and animates the same on the Replay page as it
 * does in Settings, and so a change to how a button feels is one edit.
 */

export function Toggle({
  checked,
  onChange,
  label,
  disabled,
}: {
  checked: boolean;
  onChange: (next: boolean) => void;
  label?: string;
  disabled?: boolean;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      className="toggle"
      disabled={disabled}
      onClick={() => onChange(!checked)}
    />
  );
}

export function Slider({
  value,
  min,
  max,
  step = 1,
  onChange,
  onCommit,
  format,
}: {
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange?: (value: number) => void;
  /** Called when the user lets go, for settings that restart something. */
  onCommit: (value: number) => void;
  format: (value: number) => string;
}) {
  const [local, setLocal] = useState(value);
  useEffect(() => setLocal(value), [value]);
  const fill = ((local - min) / (max - min)) * 100;
  return (
    <div className="slider">
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={local}
        style={{ ['--fill' as string]: `${fill}%` }}
        onChange={(e) => {
          const next = Number(e.target.value);
          setLocal(next);
          onChange?.(next);
        }}
        onMouseUp={() => onCommit(local)}
        onKeyUp={() => onCommit(local)}
        onTouchEnd={() => onCommit(local)}
      />
      <output>{format(local)}</output>
    </div>
  );
}

export function Segmented<T extends string | number>({
  value,
  options,
  onChange,
}: {
  value: T;
  options: { value: T; label: string }[];
  onChange: (value: T) => void;
}) {
  return (
    <div className="segmented" role="group">
      {options.map((option) => (
        <button
          key={String(option.value)}
          type="button"
          aria-pressed={option.value === value}
          onClick={() => onChange(option.value)}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}

/** One row in a settings group: text on the left, the control on the right. */
export function SettingRow({
  id,
  title,
  description,
  children,
  highlight,
  index = 0,
}: {
  id?: string;
  title: string;
  description?: ReactNode;
  children: ReactNode;
  highlight?: boolean;
  index?: number;
}) {
  return (
    <div
      className={`setting-row${highlight ? ' highlight' : ''}`}
      data-setting={id}
      style={{ ['--i' as string]: index }}
    >
      <div className="text">
        <strong>{title}</strong>
        {description && <span>{description}</span>}
      </div>
      <div className="control">{children}</div>
    </div>
  );
}

export function SettingGroup({ title, children }: { title?: string; children: ReactNode }) {
  return (
    <div className="setting-group">
      {title && <div className="setting-group-title">{title}</div>}
      {children}
    </div>
  );
}

export function PageHead({
  title,
  blurb,
  children,
}: {
  title: string;
  blurb?: ReactNode;
  children?: ReactNode;
}) {
  return (
    <div className="page-head">
      <div>
        <h1>{title}</h1>
        {blurb && <p>{blurb}</p>}
      </div>
      <span className="spacer" />
      {children}
    </div>
  );
}

export function Stat({
  label,
  value,
  sub,
  index = 0,
  className,
  children,
}: {
  label: ReactNode;
  value: ReactNode;
  sub?: ReactNode;
  index?: number;
  className?: string;
  children?: ReactNode;
}) {
  return (
    <div className={`widget${className ? ` ${className}` : ''}`} style={{ ['--i' as string]: index }}>
      <h3>{label}</h3>
      <p className="big">{value}</p>
      {sub && <p className="sub">{sub}</p>}
      {children}
    </div>
  );
}

export function Modal({
  title,
  onClose,
  children,
  wide,
  actions,
}: {
  title?: string;
  onClose: () => void;
  children: ReactNode;
  wide?: boolean;
  actions?: ReactNode;
}) {
  const titleId = useId();
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [onClose]);
  return (
    <div className="scrim" role="dialog" aria-modal="true" aria-labelledby={titleId} onClick={onClose}>
      <div className={`dialog${wide ? ' wide' : ''}`} onClick={(e) => e.stopPropagation()}>
        {title && <h2 id={titleId}>{title}</h2>}
        {children}
        {actions && <div className="dialog-actions">{actions}</div>}
      </div>
    </div>
  );
}

/** A confirm dialog that looks like the app, instead of the browser's box. */
export function Confirm({
  title,
  body,
  confirmLabel,
  danger,
  onConfirm,
  onCancel,
}: {
  title: string;
  body?: ReactNode;
  confirmLabel?: string;
  danger?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  return (
    <Modal
      title={title}
      onClose={onCancel}
      actions={
        <>
          <button className="btn" onClick={onCancel}>
            {t('common.cancel')}
          </button>
          <button className={`btn ${danger ? 'btn-danger' : 'btn-accent'}`} onClick={onConfirm} autoFocus>
            {confirmLabel ?? t('common.yes')}
          </button>
        </>
      }
    >
      {body && <p className="note">{body}</p>}
    </Modal>
  );
}

/** Runs an async action with a busy flag, for buttons that take a moment. */
export function useBusy() {
  const [busy, setBusy] = useState(false);
  const alive = useRef(true);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);
  const run = async <T,>(work: () => Promise<T>): Promise<T | undefined> => {
    setBusy(true);
    try {
      return await work();
    } finally {
      if (alive.current) setBusy(false);
    }
  };
  return { busy, run };
}

export function Kbd({ children }: { children: ReactNode }) {
  return <kbd className="key">{children}</kbd>;
}

export function Spinner() {
  return <span className="btn busy btn-ghost" style={{ pointerEvents: 'none' }} aria-label={t('common.loading')} />;
}
