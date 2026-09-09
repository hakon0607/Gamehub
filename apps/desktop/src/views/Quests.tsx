import { useEffect, useState } from 'react';
import { api, events, type Quest, type QuestBoard } from '../api';
import { timeLeftIn } from '../format';
import { PageHead } from '../ui';

function progressLabel(quest: Quest): string {
  if (quest.target >= 600) {
    const done = Math.floor(quest.progress / 60);
    const goal = Math.floor(quest.target / 60);
    return goal >= 120 ? `${(done / 60).toFixed(1)} / ${Math.round(goal / 60)} t` : `${done} / ${goal} min`;
  }
  return `${quest.progress} / ${quest.target}`;
}

function QuestRow({ quest, index }: { quest: Quest; index: number }) {
  const percent = quest.target === 0 ? 0 : Math.min(100, (quest.progress / quest.target) * 100);
  return (
    <div className={`quest${quest.complete ? ' done' : ''}`} style={{ ['--i' as string]: index }}>
      <div className="quest-head">
        <strong>
          {quest.complete ? '✓ ' : ''}
          {quest.title}
        </strong>
        <span className="quest-xp">+{quest.xp} XP</span>
      </div>
      <p className="quest-desc">{quest.description}</p>
      <div className={`progress${quest.complete ? ' done' : ''}`}>
        <span style={{ width: `${percent}%` }} />
      </div>
      <p className="quest-progress">{quest.complete ? 'Fullført' : progressLabel(quest)}</p>
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

  if (!board) return <p className="empty">Laster …</p>;
  const total = board.daily.length + board.biweekly.length + board.monthly.length;
  const percent = (board.xpIntoLevel / board.xpForLevel) * 100;

  return (
    <div className="view">
      <PageHead title="Quests" blurb="Oppdrag laget av spillene du eier og hvordan du spiller. Fremgangen følger av seg selv — ingenting å hente ut." />

      <div className="panel glow" style={{ display: 'flex', alignItems: 'center', gap: 20, marginBottom: 24, maxWidth: 640 }}>
        <div className="level-ring" style={{ ['--pct' as string]: `${percent}%` }}>
          <span>{board.level}</span>
        </div>
        <div style={{ flex: 1 }}>
          <h3 className="section-title" style={{ margin: '0 0 6px' }}>
            Nivå {board.level}
          </h3>
          <div className="progress" style={{ height: 9 }}>
            <span style={{ width: `${percent}%` }} />
          </div>
          <p className="note" style={{ marginTop: 6 }}>
            {board.xpIntoLevel.toLocaleString('nb-NO')} / {board.xpForLevel.toLocaleString('nb-NO')} XP · {board.xp.toLocaleString('nb-NO')} XP totalt
          </p>
        </div>
      </div>

      {total === 0 ? (
        <p className="empty">Oppdrag dukker opp når GameHub ser noen installerte spill. Kjør en skanning fra toppen.</p>
      ) : (
        (
          [
            ['I dag', board.daily, 'daily'],
            ['Denne uken og neste', board.biweekly, 'biweekly'],
            ['Denne måneden', board.monthly, 'monthly'],
          ] as const
        ).map(([label, quests, period]) =>
          quests.length === 0 ? null : (
            <section key={label} style={{ marginBottom: 26 }}>
              <h2 className="section-title">
                {label}
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
