import { useEffect, useRef, useState } from 'react';
import { api, type PerformanceSample } from '../api';
import { formatBytes } from '../format';
import { PageHead, Stat } from '../ui';
import { t } from '../i18n';

function History({ values, max }: { values: number[]; max: number }) {
  if (values.length < 2) return null;
  const pts = values.map((value, index) => {
    const x = (index / (values.length - 1)) * 100;
    const y = 100 - Math.min(100, (value / max) * 100);
    return `${x.toFixed(2)},${y.toFixed(2)}`;
  });
  return (
    <svg className="spark" viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true">
      <polygon points={`0,100 ${pts.join(' ')} 100,100`} />
      <polyline points={pts.join(' ')} />
    </svg>
  );
}

export function Performance({ currentGame }: { currentGame: string | null }) {
  const [sample, setSample] = useState<PerformanceSample | null>(null);
  const cpuHistory = useRef<number[]>([]);
  const memHistory = useRef<number[]>([]);

  useEffect(() => {
    let cancelled = false;
    const tick = async () => {
      try {
        const next = await api.samplePerformance();
        if (cancelled) return;
        cpuHistory.current = [...cpuHistory.current, next.cpuPercent].slice(-40);
        memHistory.current = [...memHistory.current, (next.memoryUsedBytes / Math.max(1, next.memoryTotalBytes)) * 100].slice(-40);
        setSample(next);
      } catch {
        // The next sample is two seconds away.
      }
    };
    void tick();
    const timer = window.setInterval(tick, 2000);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, []);

  if (!sample) return <p className="empty">{t('perf.reading')}</p>;
  const memPercent = (sample.memoryUsedBytes / Math.max(1, sample.memoryTotalBytes)) * 100;

  return (
    <div className="view">
      <PageHead title={t('perf.title')} blurb={t('perf.blurb')} />
      <div className="widgets">
        <Stat index={0} label={t('perf.cpu')} value={`${sample.cpuPercent.toFixed(0)} %`} sub={t('perf.cores', { name: sample.cpuName, n: sample.cpuCores })}>
          <History values={cpuHistory.current} max={100} />
        </Stat>
        <Stat index={1} label={t('perf.memory')} value={`${memPercent.toFixed(0)} %`} sub={t('perf.of', { used: formatBytes(sample.memoryUsedBytes), total: formatBytes(sample.memoryTotalBytes) })}>
          <History values={memHistory.current} max={100} />
        </Stat>
        <Stat index={2} label={t('perf.network')} value={<span style={{ fontSize: 20 }}>↓ {formatBytes(sample.networkDownBytes)}</span>} sub={t('perf.since_boot', { up: formatBytes(sample.networkUpBytes) })} />
        {sample.disks.slice(0, 3).map((disk, i) => (
          <Stat key={disk.name} index={3 + i} label={t('perf.disk', { name: disk.name })} value={`${((disk.usedBytes / Math.max(1, disk.totalBytes)) * 100).toFixed(0)} %`} sub={t('perf.used', { used: formatBytes(disk.usedBytes), total: formatBytes(disk.totalBytes) })} />
        ))}
      </div>
      {currentGame && (
        <>
          <h2 className="section-title">{currentGame}</h2>
          {sample.gameProcesses.length === 0 ? (
            <p className="note" style={{ marginBottom: 20 }}>{t('perf.outside')}</p>
          ) : (
            <div className="facts" style={{ maxWidth: 460, marginBottom: 24 }}>
              {sample.gameProcesses.map((p) => (
                <div className="fact" key={p.name}><span>{p.name}</span><span>{p.cpuPercent.toFixed(0)} % CPU · {formatBytes(p.memoryBytes)}</span></div>
              ))}
            </div>
          )}
        </>
      )}
      <p className="note">{t('perf.tools').trim()}</p>
    </div>
  );
}
