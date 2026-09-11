/** Shared formatting, so a duration reads the same on every page — in the current language. */
import { locale, t } from './i18n';

export function formatDuration(seconds: number): string {
  if (!seconds) return t('format.minutes', { n: 0 });
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.round((seconds % 3600) / 60);
  if (hours === 0) return t('format.minutes', { n: minutes });
  if (minutes === 0) return t('format.hours', { n: hours });
  return t('format.hours_minutes', { h: hours, m: minutes });
}

export function formatRelativeDay(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return t('common.never');
  const days = Math.floor((Date.now() - date.getTime()) / 86_400_000);
  if (days <= 0) return t('common.today');
  if (days === 1) return t('common.yesterday');
  if (days < 7) return t('common.days_ago', { n: days });
  return date.toLocaleDateString(locale());
}

export function monthKey(date: Date): string {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}`;
}

export function monthLabel(key: string): string {
  const [year, month] = key.split('-').map(Number);
  if (!year || !month) return key;
  return new Date(year, month - 1, 1).toLocaleDateString(locale(), { month: 'long', year: 'numeric' });
}

/** How long is left in a quest period, in plain words. */
export function timeLeftIn(period: 'daily' | 'biweekly' | 'monthly', now = new Date()): string {
  const end = periodEnd(period, now);
  const ms = end.getTime() - now.getTime();
  if (ms <= 0) return t('common.done');
  const minutes = Math.floor(ms / 60_000);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);
  if (days >= 2) return t('format.days_left', { n: days });
  if (hours >= 24) return t('format.day_hours_left', { h: hours - 24 });
  if (hours >= 1) return t('format.hours_minutes_left', { h: hours, m: minutes % 60 });
  return t('format.minutes_left', { m: minutes });
}

function periodEnd(period: 'daily' | 'biweekly' | 'monthly', now: Date): Date {
  if (period === 'daily') {
    const end = new Date(now);
    end.setHours(24, 0, 0, 0);
    return end;
  }
  if (period === 'monthly') return new Date(now.getFullYear(), now.getMonth() + 1, 1);
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
  return date.toLocaleString(locale(), { dateStyle: 'medium', timeStyle: 'short' });
}

export function formatDate(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return date.toLocaleDateString(locale());
}

export function formatNumber(n: number): string {
  return n.toLocaleString(locale());
}

/** "30 s", "2 min", "1 min 30 s" — for buffer and clip lengths. */
export function formatSeconds(seconds: number): string {
  if (seconds < 60) return t('format.seconds', { n: seconds });
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return s === 0 ? t('format.minutes', { n: m }) : t('format.minutes_seconds', { m, s });
}

export function formatPlaytime(seconds: number | null): string {
  if (!seconds) return '—';
  const hours = seconds / 3600;
  return hours >= 1 ? t('format.hours_long', { n: hours.toFixed(1) }) : t('format.minutes_long', { n: Math.round(seconds / 60) });
}
