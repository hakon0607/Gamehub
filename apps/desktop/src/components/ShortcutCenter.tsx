import { useCallback, useEffect, useState } from 'react';
import { api, type ShortcutEntry } from '../api';
import { SettingGroup } from '../ui';
import { t, tr, type Key } from '../i18n';
import en from '../i18n/en.json';

/** Turns a keydown into the accelerator string Tauri expects. */
function accelerator(event: KeyboardEvent): string | null {
  const parts: string[] = [];
  if (event.ctrlKey) parts.push('Ctrl');
  if (event.altKey) parts.push('Alt');
  if (event.shiftKey) parts.push('Shift');
  if (event.metaKey) parts.push('Super');
  const key = event.key;
  if (['Control', 'Alt', 'Shift', 'Meta', 'OS'].includes(key)) return null;
  const named = key === ' ' ? 'Space' : key.length === 1 ? key.toUpperCase() : key.charAt(0).toUpperCase() + key.slice(1);
  parts.push(named);
  return parts.join('+');
}

/** The action's label in the current language, from the `sc.*` keys. */
export function shortcutLabel(entry: ShortcutEntry): string {
  const key = `sc.${entry.action}` as Key;
  return key in en ? t(key) : entry.label;
}

/**
 * Every shortcut GameHub has, from the backend registry — so the list can
 * never claim a shortcut that does not exist or miss one that does.
 */
export function ShortcutCenter({
  onToast,
  filter,
}: {
  onToast: (title: string, body?: string) => void;
  /** Only show actions whose id is in this list, for the Freeze page etc. */
  filter?: string[];
}) {
  const [shortcuts, setShortcuts] = useState<ShortcutEntry[]>([]);
  const [listening, setListening] = useState<string | null>(null);

  const load = useCallback(() => void api.getShortcuts().then(setShortcuts), []);
  useEffect(load, [load]);

  useEffect(() => {
    if (!listening) return;
    const onKey = async (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();
      if (event.key === 'Escape') {
        setListening(null);
        return;
      }
      // Backspace or Delete while listening removes the shortcut entirely.
      if (event.key === 'Backspace' || event.key === 'Delete') {
        const action = listening;
        setListening(null);
        try {
          await api.setShortcut(action, '');
          load();
          window.dispatchEvent(new Event('shortcuts-changed'));
        } catch (error) {
          onToast(t('sc.not_set'), tr(error));
        }
        return;
      }
      const binding = accelerator(event);
      if (!binding) return;
      const action = listening;
      setListening(null);
      try {
        await api.setShortcut(action, binding);
        load();
        window.dispatchEvent(new Event('shortcuts-changed'));
      } catch (error) {
        onToast(t('sc.not_set'), tr(error));
      }
    };
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  }, [listening, load, onToast]);

  const shown = filter ? shortcuts.filter((s) => filter.includes(s.action)) : shortcuts;
  const global = shown.filter((s) => s.scope === 'global');
  const app = shown.filter((s) => s.scope === 'app');

  const row = (shortcut: ShortcutEntry) => (
    <div className="shortcut-row" key={shortcut.action}>
      <span className="label">{shortcutLabel(shortcut)}</span>
      <span className="shortcut-scope">{shortcut.scope === 'global' ? t('sc.scope_global') : t('sc.scope_app')}</span>
      <button
        className={`keycap${listening === shortcut.action ? ' listening' : ''}`}
        onClick={() => setListening(shortcut.action)}
      >
        {listening === shortcut.action ? t('sc.press') : shortcut.binding || t('sc.none')}
      </button>
      {shortcut.binding && (
        <button
          className="btn sm btn-ghost"
          title={t('sc.remove_hint')}
          onClick={async () => {
            try {
              await api.setShortcut(shortcut.action, '');
              load();
              window.dispatchEvent(new Event('shortcuts-changed'));
            } catch (error) {
              onToast(t('sc.not_set'), tr(error));
            }
          }}
        >
          {t('sc.remove')}
        </button>
      )}
      {shortcut.binding !== shortcut.defaultBinding && (
        <button
          className="btn sm btn-ghost"
          onClick={async () => {
            await api.resetShortcut(shortcut.action);
            load();
            window.dispatchEvent(new Event('shortcuts-changed'));
          }}
        >
          {t('sc.reset')}
        </button>
      )}
    </div>
  );

  return (
    <>
      {global.length > 0 && <SettingGroup title={t('sc.global')}>{global.map(row)}</SettingGroup>}
      {app.length > 0 && <SettingGroup title={t('sc.app')}>{app.map(row)}</SettingGroup>}
    </>
  );
}
