import { useEffect, useRef, useState } from 'react';
import { api, type PerformanceSample } from '../api';
import { formatBytes } from '../format';
import { PageHead, Stat } from '../ui';

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

  if (!sample) return <p className="empty">Leser systemet …</p>;
  const memPercent = (sample.memoryUsedBytes / Math.max(1, sample.memoryTotalBytes)) * 100;

  return (
    <div className="view">
      <PageHead title="Ytelse" blurb="Ekte tall fra Windows. Bilder per sekund og GPU-temperatur vises ikke — GameHub viser heller ingenting enn et tall som ikke stemmer." />
      <div className="widgets">
        <Stat index={0} label="Prosessor" value={`${sample.cpuPercent.toFixed(0)} %`} sub={`${sample.cpuName} · ${sample.cpuCores} kjerner`}>
          <History values={cpuHistory.current} max={100} />
        </Stat>
        <Stat index={1} label="Minne" value={`${memPercent.toFixed(0)} %`} sub={`${formatBytes(sample.memoryUsedBytes)} av ${formatBytes(sample.memoryTotalBytes)}`}>
          <History values={memHistory.current} max={100} />
        </Stat>
        <Stat index={2} label="Nettverk" value={<span style={{ fontSize: 20 }}>↓ {formatBytes(sample.networkDownBytes)}</span>} sub={`↑ ${formatBytes(sample.networkUpBytes)} siden oppstart`} />
        {sample.disks.slice(0, 3).map((disk, i) => (
          <Stat key={disk.name} index={3 + i} label={`Disk ${disk.name}`} value={`${((disk.usedBytes / Math.max(1, disk.totalBytes)) * 100).toFixed(0)} %`} sub={`${formatBytes(disk.usedBytes)} av ${formatBytes(disk.totalBytes)} brukt`} />
        ))}
      </div>
      {currentGame && (
        <>
          <h2 className="section-title">{currentGame}</h2>
          {sample.gameProcesses.length === 0 ? (
            <p className="note" style={{ marginBottom: 20 }}>Spillet kjører, men prosessene ligger utenfor mappen GameHub kjenner til.</p>
          ) : (
            <div className="facts" style={{ maxWidth: 460, marginBottom: 24 }}>
              {sample.gameProcesses.map((p) => (
                <div className="fact" key={p.name}><span>{p.name}</span><span>{p.cpuPercent.toFixed(0)} % CPU · {formatBytes(p.memoryBytes)}</span></div>
              ))}
            </div>
          )}
        </>
      )}
      <p className="note">{sample.unavailableNote} Vil du ha FPS og temperaturer, gjør MSI Afterburner med RivaTuner eller Nvidias eget overlegg den jobben ordentlig.</p>
    </div>
  );
}
