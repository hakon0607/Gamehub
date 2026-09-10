import { useEffect, useState } from 'react';
import { api, events, type Quest, type QuestBoard, type QuestGoal, type QuestPeriod } from '../api';
import { formatNumber, timeLeftIn } from '../format';
import { t } from '../i18n';
import { PageHead } from '../ui';

/**
 * A quest's words come from its goal, not from the backend: the generator
 * knows what it asks for, and the language is the interface's business.
 */
export function questText(goal: QuestGoal | undefined, period: QuestPeriod, fallback?: { title: string; description: string }): { title: string; description: string } {
  const span = t(`quests.span_${period}`);
  // A quest saved by an older version has no goal; its stored words are all there is.
  if (!goal) return fallback ?? { title: '', description: '' };
  switch (goal.kind) {
    case 'playGame':
      return { title: goal.gameName, description: t('quests.d_play_game', { game: goal.gameName, minutes: Math.round(goal.seconds / 60), span }) };
    case 'revisit':
      return { title: t('quests.t_revisit'), description: t('quests.d_revisit', { game: goal.gameName, minutes: Math.round(goal.seconds / 60), span }) };
    case 'distinctGames':
      return { title: t('quests.t_distinct'), description: t('quests.d_distinct', { count: goal.count, span }) };
    case 'playOnDays':
      return { title: t('quests.t_days'), description: t('quests.d_days', { days: goal.days, span }) };
    case 'totalPlaytime':
      return {
        title: t('quests.t_total'),
        description:
          goal.seconds >= 3600
            ? t('quests.d_total_hours', { hours: Math.round(goal.seconds / 3600), span })
            : t('quests.d_total_minutes', { minutes: Math.round(goal.seconds / 60), span }),
      };
    case 'trySomethingNew':
      return { title: t('quests.t_new'), description: t('quests.d_new', { minutes: Math.round(goal.seconds / 60), span }) };
  }
}

function progressLabel(quest: Quest): string {
  if (quest.target >= 600) {
    const done = Math.floor(quest.progress / 60);
    const goal = Math.floor(quest.target / 60);
    return goal >= 120 ? t('quests.progress_hours', { done: (done / 60).toFixed(1), goal: Math.round(goal / 60) }) : t('quests.progress_minutes', { done, goal });
  }
  return `${quest.progress} / ${quest.target}`;
}

function QuestRow({ quest, index }: { quest: Quest; index: number }) {
  const percent = quest.target === 0 ? 0 : Math.min(100, (quest.progress / quest.target) * 100);
  const text = questText(quest.goal, quest.period, { title: quest.title, description: quest.description });
  return (
    <div className={`quest${quest.complete ? ' done' : ''}`} style={{ ['--i' as string]: index }}>
      <div className="quest-head">
        <strong>
          {quest.complete ? '✓ ' : ''}
          {text.title}
        </strong>
        <span className="quest-xp">{t('quests.plus_xp', { n: quest.xp })}</span>
      </div>
      <p className="quest-desc">{text.description}</p>
      <div className={`progress${quest.complete ? ' done' : ''}`}>
        <span style={{ width: `${percent}%` }} />
      </div>
      <p className="quest-progress">{quest.complete ? t('quests.complete') : progressLabel(quest)}</p>
    </div>
  );
}

export function Quests() {
  const [board, setBoard] = useState<QuestBoard | null>(null);
  const [, setNow] = useState(new Date());

  useEffect(() => {
    const load = () => void api.getQuests().then(setBoard);
    load();
    const pending = events.onActivityUpdated(load);
    const ticking = window.setInterval(() => {
      load();
      setNow(new Date());
    }, 60_000);
    return () => {
      window.clearInterval(ticking);
      void pending.then((off) => off());
    };
  }, []);

  if (!board) return <p className="empty">{t('common.loading')}</p>;
  const total = board.daily.length + board.biweekly.length + board.monthly.length;
  const percent = (board.xpIntoLevel / board.xpForLevel) * 100;

  return (
    <div className="view">
      <PageHead title={t('quests.title')} blurb={t('quests.blurb')} />

      <div className="panel glow" style={{ display: 'flex', alignItems: 'center', gap: 20, marginBottom: 24, maxWidth: 640 }}>
        <div className="level-ring" style={{ ['--pct' as string]: `${percent}%` }}>
          <span>{board.level}</span>
        </div>
        <div style={{ flex: 1 }}>
          <h3 className="section-title" style={{ margin: '0 0 6px' }}>
            {t('quests.level', { n: board.level })}
          </h3>
          <div className="progress" style={{ height: 9 }}>
            <span style={{ width: `${percent}%` }} />
          </div>
          <p className="note" style={{ marginTop: 6 }}>
            {t('quests.xp', { into: formatNumber(board.xpIntoLevel), per: formatNumber(board.xpForLevel), total: formatNumber(board.xp) })}
          </p>
        </div>
      </div>

      {total === 0 ? (
        <p className="empty">{t('quests.empty')}</p>
      ) : (
        (
          [
            ['quests.daily', board.daily, 'daily'],
            ['quests.biweekly', board.biweekly, 'biweekly'],
            ['quests.monthly', board.monthly, 'monthly'],
          ] as const
        ).map(([label, quests, period]) =>
          quests.length === 0 ? null : (
            <section key={label} style={{ marginBottom: 26 }}>
              <h2 className="section-title">
                {t(label)}
                <span className="countdown">{timeLeftIn(period)}</span>
              </h2>
              <div className="quest-list">
                {quests.map((quest, index) => (
                  <QuestRow key={quest.id} quest={quest} index={index} />
                ))}
              </div>
            </section>
          ),
        )
      )}
    </div>
  );
}
