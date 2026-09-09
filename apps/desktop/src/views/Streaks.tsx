import type { ActivitySummary } from '../api';
import { formatDuration } from '../format';
import { PageHead, Stat } from '../ui';

export function Streaks({ activity }: { activity: ActivitySummary | null }) {
  if (!activity) return <p className="empty">Laster …</p>;
  const { streaks } = activity;
  const [bestMonth, bestSeconds] = streaks.bestMonth ?? [null, 0];

  return (
    <div className="view">
      <PageHead title="Streaks" blurb="En dag teller når du har spilt lenge nok — terskelen står i Innstillinger → Personvern. Ingenting å starte eller stoppe." />
      {!activity.trackingEnabled && (
        <div className="notice">Aktivitetssporing er slått av i Innstillinger → Personvern, så ingenting nytt registreres.</div>
      )}
      <div className="streak-grid" style={{ marginBottom: 24 }}>
        <Stat index={0} label="Nåværende streak" value={<><span className="flame">🔥</span> {streaks.current}</>} sub={streaks.current === 1 ? 'dag på rad' : 'dager på rad'} />
        <Stat index={1} label="Lengste streak" value={`🏆 ${streaks.longest}`} sub={streaks.longest === 1 ? 'dag' : 'dager'} />
        <Stat index={2} label="Spilledager" value={`🎮 ${streaks.totalDays}`} sub="dager som telte" />
        <Stat index={3} label="Total spilletid" value={`${Math.round(streaks.totalSeconds / 3600)} t`} sub={formatDuration(streaks.totalSeconds)} />
      </div>
      {streaks.currentStreakGames.length > 0 && (
        <>
          <h2 className="section-title">Spilt i denne streaken</h2>
          <div className="filters">
            {streaks.currentStreakGames.map((name) => (
              <span key={name} className="chip">{name}</span>
            ))}
          </div>
        </>
      )}
      {bestMonth && (
        <>
          <h2 className="section-title">Beste måned</h2>
          <p className="note">{bestMonth} — {formatDuration(bestSeconds)}.</p>
        </>
      )}
    </div>
  );
}
