/** Shared formatting, so a duration reads the same on every page. */

export function formatDuration(seconds: number): string {
  if (!seconds) return '0 min';
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.round((seconds % 3600) / 60);
  if (hours === 0) return `${minutes} min`;
  if (minutes === 0) return `${hours} t`;
  return `${hours} t ${minutes} min`;
}

export function formatRelativeDay(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return 'Aldri';
  const days = Math.floor((Date.now() - date.getTime()) / 86_400_000);
  if (days <= 0) return 'I dag';
  if (days === 1) return 'I går';
  if (days < 7) return `${days} dager siden`;
  return date.toLocaleDateString('nb-NO');
}

export function monthKey(date: Date): string {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}`;
}

export function monthLabel(key: string): string {
  const [year, month] = key.split('-').map(Number);
  if (!year || !month) return key;
  return new Date(year, month - 1, 1).toLocaleDateString('nb-NO', { month: 'long', year: 'numeric' });
}

/**
 * How long is left in a quest period, in plain words.
 *
 * Daily quests reset at midnight, fortnightly and monthly at the end of their
 * window — the same boundaries the backend uses to decide which quests exist,
 * so the countdown and the board can never disagree.
 */
export function timeLeftIn(period: 'daily' | 'biweekly' | 'monthly', now = new Date()): string {
  const end = periodEnd(period, now);
  const ms = end.getTime() - now.getTime();
  if (ms <= 0) return 'ferdig';

  const minutes = Math.floor(ms / 60_000);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);

  if (days >= 2) return `${days} dager igjen`;
  if (hours >= 24) return `1 dag ${hours - 24} t igjen`;
  if (hours >= 1) return `${hours} t ${minutes % 60} min igjen`;
  return `${minutes} min igjen`;
}

function periodEnd(period: 'daily' | 'biweekly' | 'monthly', now: Date): Date {
  if (period === 'daily') {
    const end = new Date(now);
    end.setHours(24, 0, 0, 0);
    return end;
  }
  if (period === 'monthly') {
    return new Date(now.getFullYear(), now.getMonth() + 1, 1);
  }
  // Fortnights are counted from the start of the year, matching the backend.
  const startOfYear = new Date(now.getFullYear(), 0, 1);
  const dayOfYear = Math.floor((now.getTime() - startOfYear.getTime()) / 86_400_000);
  const fortnight = Math.floor(dayOfYear / 14);
  const end = new Date(startOfYear);
  end.setDate(startOfYear.getDate() + (fortnight + 1) * 14);
  end.setHours(0, 0, 0, 0);
  return end;
}

export function formatBytes(bytes: number): string {
  if (!bytes) return '0 B';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 ** 2) return `${Math.round(bytes / 1024)} kB`;
  if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(bytes < 10 * 1024 ** 2 ? 1 : 0)} MB`;
  return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
}

export function formatDateTime(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return date.toLocaleString('nb-NO', { dateStyle: 'medium', timeStyle: 'short' });
}

export function formatClock(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  return `${m}:${String(s).padStart(2, '0')}`;
}

/** "30 sek", "2 min", "1 min 30 sek" — for buffer and clip lengths. */
export function formatSeconds(seconds: number): string {
  if (seconds < 60) return `${seconds} sek`;
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return s === 0 ? `${m} min` : `${m} min ${s} sek`;
}
