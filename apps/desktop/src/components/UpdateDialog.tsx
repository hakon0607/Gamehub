import { useState } from 'react';
import type { Update } from '@tauri-apps/plugin-updater';
import { downloadAndInstall, describeUpdateError, type UpdateStatus } from '../updater';
import { Modal } from '../ui';

/**
 * The "new version available" prompt.
 *
 * Shown in exactly two situations: the quiet check at startup found a version
 * the user has not already put off, or they pressed "Se etter oppdateringer"
 * in Settings. Nothing else opens it, so GameHub never nags.
 */
export function UpdateDialog({
  status,
  update,
  onLater,
  onClose,
  onError,
}: {
  status: UpdateStatus;
  update: Update;
  onLater: (version: string) => void;
  onClose: () => void;
  onError: (message: string) => void;
}) {
  const [installing, setInstalling] = useState(false);
  const [percent, setPercent] = useState(0);
  const [indeterminate, setIndeterminate] = useState(false);

  const install = async () => {
    setInstalling(true);
    try {
      await downloadAndInstall(update, (value) => {
        if (value < 0) setIndeterminate(true);
        else setPercent(value);
      });
    } catch (error) {
      setInstalling(false);
      setPercent(0);
      onError(describeUpdateError(error));
    }
  };

  return (
    <Modal
      title={`GameHub ${status.version} er klar`}
      onClose={() => !installing && onClose()}
      actions={
        installing ? null : (
          <>
            <button className="btn" onClick={() => onLater(status.version ?? '')}>
              Senere
            </button>
            <button className="btn btn-accent" onClick={() => void install()} autoFocus>
              Oppdater nå
            </button>
          </>
        )
      }
    >
      <p className="version-note">
        Du har {status.currentVersion}
        {status.date ? ` · publisert ${new Date(status.date).toLocaleDateString()}` : ''}
      </p>
      {status.notes && <div className="notes">{status.notes}</div>}
      <p className="note">
        Oppdateringen lastes ned, sjekkes mot signaturen og installeres. GameHub starter på nytt etterpå.
        Biblioteket, innstillingene, klippene og frysepunktene dine rører den ikke.
      </p>
      {installing && (
        <div style={{ marginTop: 16 }}>
          <div className="progress" style={{ height: 9 }}>
            <span style={{ width: indeterminate ? '100%' : `${percent}%` }} />
          </div>
          <p className="note" style={{ marginTop: 8 }}>
            {indeterminate ? 'Laster ned …' : percent < 100 ? `Laster ned … ${percent} %` : 'Installerer og starter på nytt …'}
          </p>
        </div>
      )}
    </Modal>
  );
}
