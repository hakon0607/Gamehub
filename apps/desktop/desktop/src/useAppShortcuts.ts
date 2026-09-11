import { useEffect, useRef } from 'react';
import { api, type ShortcutEntry } from './api';
import { acceleratorFor, isTypingTarget, matchAction } from './shortcutMatch';

/**
 * Makes the app-scope shortcuts actually do something.
 *
 * Global shortcuts (screenshot, save replay, open GameHub) are registered with
 * Windows by the backend, because they have to work while a game is in front.
 * The app-scope ones — search, the navigation keys, rescan — are the window's
 * job, and nothing was listening for them: they appeared in Settings, could be
 * rebound, and then did nothing at all when pressed.
 *
 * The bindings come from the same backend registry the Settings list is built
 * from, so a rebind takes effect immediately and the two can never disagree.
 */
export function useAppShortcuts(run: (action: string) => void) {
  const shortcuts = useRef<ShortcutEntry[]>([]);
  const handler = useRef(run);
  handler.current = run;

  useEffect(() => {
    const load = () => void api.getShortcuts().then((list) => (shortcuts.current = list));
    load();

    // Rebinding in Settings must take effect without a restart.
    window.addEventListener('shortcuts-changed', load);

    const onKey = (event: KeyboardEvent) => {
      const element = event.target as HTMLElement | null;
      const typing = isTypingTarget(element?.tagName, element?.isContentEditable === true);
      const action = matchAction(acceleratorFor(event), shortcuts.current, typing);
      if (!action) return;
      event.preventDefault();
      handler.current(action);
    };

    window.addEventListener('keydown', onKey);
    return () => {
      window.removeEventListener('keydown', onKey);
      window.removeEventListener('shortcuts-changed', load);
    };
  }, []);
}
