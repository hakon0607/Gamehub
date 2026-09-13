/**
 * Deciding which shortcut a keystroke means.
 *
 * Deliberately in its own file with no imports, so it can be tested directly.
 * The subtle rules live here — what counts as typing, when a bare key may fire
 * — and those are exactly the rules worth having tests for.
 */

export interface BindingLike {
  action: string;
  scope: 'global' | 'app';
  binding: string;
}

/** The accelerator string for a keyboard event, in the backend's format. */
export function acceleratorFor(event: {
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
  key: string;
}): string {
  const parts: string[] = [];
  if (event.ctrlKey) parts.push('Ctrl');
  if (event.altKey) parts.push('Alt');
  if (event.shiftKey) parts.push('Shift');
  if (event.metaKey) parts.push('Super');

  const key = event.key;
  if (['Control', 'Alt', 'Shift', 'Meta', 'OS'].includes(key)) return '';

  const named =
    key === ' '
      ? 'Space'
      : key.length === 1
        ? key.toUpperCase()
        : key.charAt(0).toUpperCase() + key.slice(1);

  parts.push(named);
  return parts.join('+');
}

/** True when the keystroke belongs to whatever the user is typing in. */
export function isTypingTarget(tagName: string | undefined, contentEditable: boolean): boolean {
  if (contentEditable) return true;
  return tagName === 'INPUT' || tagName === 'TEXTAREA' || tagName === 'SELECT';
}

const HAS_MODIFIER = /^(Ctrl|Alt|Shift|Super)\+/i;

/**
 * Chooses the action for a keystroke, or null for none.
 *
 * Global shortcuts are ignored here on purpose: Windows already delivers those
 * through the backend, and handling them again in the window would fire them
 * twice whenever GameHub happened to have focus.
 */
export function matchAction(
  accelerator: string,
  shortcuts: BindingLike[],
  typing: boolean,
): string | null {
  if (!accelerator) return null;

  const hit = shortcuts.find(
    (entry) => entry.scope === 'app' && entry.binding.toLowerCase() === accelerator.toLowerCase(),
  );
  if (!hit) return null;

  // While the user is typing, only a shortcut with a modifier may fire — a bare
  // F-key or letter has to reach the field they are typing in.
  if (typing && !HAS_MODIFIER.test(accelerator)) return null;

  return hit.action;
}
