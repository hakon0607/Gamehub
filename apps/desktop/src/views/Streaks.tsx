import type { ActivitySummary } from '../api';
import { formatDuration } from '../format';
import { t } from '../i18n';
import { PageHead, Stat } from '../ui';

export function Streaks({ activity }: { activity: ActivitySummary | null }) {
  if (!activity) return <p className="empty">{t('common.loading')}</p>;
  const { streaks } = activity;
  const [bestMonth, bestSeconds] = streaks.bestMonth ?? [null, 0];

  return (
    <div className="view">
      <PageHead title={t('streaks.title')} blurb={t('streaks.blurb')} />
      {!activity.trackingEnabled && <div className="notice">{t('streaks.tracking_off')}</div>}
      <div className="streak-grid" style={{ marginBottom: 24 }}>
        <Stat index={0} label={t('streaks.current')} value={<><span className="flame">🔥</span> {streaks.current}</>} sub={streaks.current === 1 ? t('streaks.day_in_row') : t('streaks.days_in_row')} />
        <Stat index={1} label={t('streaks.longest')} value={`🏆 ${streaks.longest}`} sub={streaks.longest === 1 ? t('streaks.day') : t('streaks.days')} />
        <Stat index={2} label={t('streaks.gaming_days')} value={`🎮 ${streaks.totalDays}`} sub={t('streaks.days_counted')} />
        <Stat index={3} label={t('streaks.total')} value={t('format.hours', { n: Math.round(streaks.totalSeconds / 3600) })} sub={formatDuration(streaks.totalSeconds)} />
      </div>
      {streaks.currentStreakGames.length > 0 && (
        <>
          <h2 className="section-title">{t('streaks.played_in_streak')}</h2>
          <div className="filters">
            {streaks.currentStreakGames.map((name) => (
              <span key={name} className="chip">{name}</span>
            ))}
          </div>
        </>
      )}
      {bestMonth && (
        <>
          <h2 className="section-title">{t('streaks.best_month')}</h2>
          <p className="note">{bestMonth} — {formatDuration(bestSeconds)}.</p>
        </>
      )}
    </div>
  );
}
