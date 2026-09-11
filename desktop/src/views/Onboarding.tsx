import type { LauncherStatus } from '@gamehub/shared';
import { SOURCE_LABELS } from '@gamehub/shared';
import { t } from '../i18n';

export function Onboarding({ launchers, gameCount, scanning, onRescan, onDone }: { launchers: LauncherStatus[]; gameCount: number; scanning: boolean; onRescan: () => void; onDone: () => void }) {
  const found = launchers.filter((l) => l.detected);
  return (
    <div className="onboarding">
      <div className="onboarding-inner">
        <div className="brand" style={{ justifyContent: 'center', fontSize: 20 }}>
          <span className="brand-mark">◆</span> GameHub
        </div>
        <h1 style={{ marginBottom: 6 }}>{t('onb.welcome')}</h1>
        <p className="note" style={{ margin: '0 auto 24px' }}>
          {scanning || launchers.length === 0 ? t('onb.looking') : t('onb.found_blurb')}
        </p>
        <div style={{ textAlign: 'left', marginBottom: 20 }}>
          {launchers.map((launcher, index) => (
            <div key={launcher.source} className="launcher-line" style={{ ['--i' as string]: index }}>
              <span>{SOURCE_LABELS[launcher.source]}</span>
              <span className={launcher.detected ? 'found' : 'missing'}>{launcher.detected ? t('onb.games', { n: launcher.gameCount }) : t('onb.not_installed')}</span>
            </div>
          ))}
        </div>
        {found.length > 0 && <p style={{ fontSize: 22, fontWeight: 700, margin: '0 0 20px' }}>{t('onb.found', { n: gameCount })}</p>}
        <div className="row" style={{ justifyContent: 'center' }}>
          <button className="btn btn-accent" onClick={onDone} disabled={scanning}>{t('onb.continue')}</button>
          <button className="btn" onClick={onRescan} disabled={scanning}>{t('onb.rescan')}</button>
        </div>
      </div>
    </div>
  );
}
