import { formatDate, formatSize } from '@/lib/format';
import type { Release } from '@/lib/types';

const KIND_LABEL: Record<Release['files'][number]['kind'], string> = {
  installer: 'Installer',
  portable: 'Portabel',
  other: 'Fil',
};

export function ReleaseCard({ release, latest }: { release: Release; latest?: boolean }) {
  return (
    <article className="release">
      <div className="release-head">
        <span className="release-version">{release.version}</span>
        {release.name && <span className="release-name">{release.name}</span>}
        {latest && <span className="tag tag-latest">Nyeste</span>}
        <span className="release-date">{formatDate(release.publishedAt)}</span>
      </div>

      {release.description && <p className="release-body">{release.description}</p>}

      {release.files.length > 0 && (
        <div className="files">
          {release.files.map((file) => (
            <a className="file" key={file.url} href={file.url} download>
              <span aria-hidden="true">↓</span>
              <span>
                {file.name}
                <br />
                <small>
                  {KIND_LABEL[file.kind]} · {formatSize(file.size)}
                </small>
              </span>
            </a>
          ))}
        </div>
      )}
    </article>
  );
}
