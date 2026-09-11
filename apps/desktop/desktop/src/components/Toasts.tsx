export interface Toast {
  id: string;
  title: string;
  body?: string;
}

export function Toasts({ toasts }: { toasts: Toast[] }) {
  if (toasts.length === 0) return null;
  return (
    <div className="toasts" aria-live="polite">
      {toasts.map((toast) => (
        <div key={toast.id} className="toast">
          <strong>{toast.title}</strong>
          {toast.body && <span>{toast.body}</span>}
        </div>
      ))}
    </div>
  );
}
