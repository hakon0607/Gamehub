import { useEffect, useMemo, useState } from 'react';
import { api, type DaySummary } from '../api';
import { formatDuration, monthKey, monthLabel } from '../format';
import { Stat } from '../ui';
import { t, tn } from '../i18n';


export function Calendar() {
  const [month, setMonth] = useState(() => monthKey(new Date()));
  const [days, setDays] = useState<DaySummary[]>([]);
  const [selected, setSelected] = useState<DaySummary | null>(null);

  useEffect(() => {
    void api.getCalendarMonth(month).then((result) => {
      setDays(result);
      setSelected(null);
    });
  }, [month]);

  const byDate = useMemo(() => new Map(days.map((d) => [d.date, d])), [days]);
  const busiest = useMemo(() => Math.max(1, ...days.map((d) => d.seconds)), [days]);
  const [year, monthNumber] = month.split('-').map(Number);
  const first = new Date(year ?? 2026, (monthNumber ?? 1) - 1, 1);
  const daysInMonth = new Date(year ?? 2026, monthNumber ?? 1, 0).getDate();
  const leading = (first.getDay() + 6) % 7;
  const total = days.reduce((sum, d) => sum + d.seconds, 0);
  const mostPlayed = useMemo(() => {
    const totals = new Map<string, number>();
    for (const day of days) for (const [name, seconds] of day.games) totals.set(name, (totals.get(name) ?? 0) + seconds);
    return [...totals.entries()].sort((a, b) => b[1] - a[1])[0] ?? null;
  }, [days]);
  const shift = (delta: number) => setMonth(monthKey(new Date(year ?? 2026, (monthNumber ?? 1) - 1 + delta, 1)));
  const todayIso = new Date().toISOString().slice(0, 10);
  const weekdays = t('cal.weekdays').split(',');

  return (
    <div className="view">
      <div className="cal-head">
        <h1 style={{ fontSize: 24, fontWeight: 700, flex: 1, textTransform: 'capitalize' }}>{monthLabel(month)}</h1>
        <button className="btn icon" onClick={() => shift(-1)} aria-label={t('cal.prev')}>←</button>
        <button className="btn" onClick={() => setMonth(monthKey(new Date()))}>{t('cal.today')}</button>
        <button className="btn icon" onClick={() => shift(1)} aria-label={t('cal.next')}>→</button>
      </div>

      <div className="widgets">
        <Stat index={0} label={t('cal.this_month')} value={formatDuration(total)} sub={tn('cal.across_day', 'cal.across_days', days.length)} />
        {mostPlayed && <Stat index={1} label={t('cal.most_played')} value={<span style={{ fontSize: 20 }}>{mostPlayed[0]}</span>} sub={formatDuration(mostPlayed[1])} />}
      </div>

      <div className="cal-grid" style={{ marginBottom: 8 }}>
        {weekdays.map((day) => <div key={day} className="cal-label">{day}</div>)}
      </div>
      <div className="cal-grid">
        {Array.from({ length: leading }, (_, i) => <button key={`pad-${i}`} className="cal-day" disabled aria-hidden="true" />)}
        {Array.from({ length: daysInMonth }, (_, i) => {
          const date = `${month}-${String(i + 1).padStart(2, '0')}`;
          const day = byDate.get(date);
          return (
            <button
              key={date}
              className={`cal-day${day ? ' has-play' : ''}${date === todayIso ? ' today' : ''}`}
              onClick={() => setSelected(day ?? null)}
              disabled={!day}
              title={day ? `${formatDuration(day.seconds)} ${date}` : date}
            >
              {day && <span className="heat" style={{ opacity: 0.12 + (day.seconds / busiest) * 0.35 }} />}
              <span style={{ position: 'relative' }}>{i + 1}</span>
              {day && <span className="mins">{formatDuration(day.seconds)}</span>}
            </button>
          );
        })}
      </div>

      {selected && (
        <div style={{ marginTop: 22 }} className="view">
          <h2 className="section-title">{selected.date}</h2>
          <div className="facts" style={{ maxWidth: 420 }}>
            <div className="fact"><span>{t('cal.played')}</span><span>{formatDuration(selected.seconds)}</span></div>
            <div className="fact"><span>{t('cal.sessions')}</span><span>{selected.sessionCount}</span></div>
            {selected.games.map(([name, seconds]) => (
              <div key={name} className="fact"><span>{name}</span><span>{formatDuration(seconds)}</span></div>
            ))}
          </div>
        </div>
      )}
      {days.length === 0 && <p className="empty" style={{ marginTop: 20 }}>{t('cal.empty')}</p>}
    </div>
  );
}
