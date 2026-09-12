import type { ReactNode } from 'react';

/**
 * Just enough Markdown for the legal documents: headings, paragraphs, lists,
 * tables, bold, inline code and links. Built as React elements rather than
 * with dangerouslySetInnerHTML — the content is ours, but a legal page is
 * the last place to leave an HTML injection lying around, and the app's CSP
 * is strict about what may run.
 */
function inline(text: string, keyBase: string): ReactNode[] {
  const out: ReactNode[] = [];
  const pattern = /(\*\*[^*]+\*\*|`[^`]+`|https?:\/\/[^\s)]+)/g;
  let last = 0;
  let match: RegExpExecArray | null;
  let i = 0;
  while ((match = pattern.exec(text))) {
    if (match.index > last) out.push(text.slice(last, match.index));
    const token = match[0];
    const key = `${keyBase}-${i++}`;
    if (token.startsWith('**')) out.push(<strong key={key}>{token.slice(2, -2)}</strong>);
    else if (token.startsWith('`')) out.push(<code key={key}>{token.slice(1, -1)}</code>);
    else out.push(
      <a key={key} href={token} target="_blank" rel="noreferrer noopener">
        {token}
      </a>,
    );
    last = match.index + token.length;
  }
  if (last < text.length) out.push(text.slice(last));
  return out;
}

const cells = (row: string) =>
  row
    .trim()
    .replace(/^\||\|$/g, '')
    .split('|')
    .map((c) => c.trim());

export function Markdown({ text }: { text: string }) {
  const lines = text.split('\n');
  const blocks: ReactNode[] = [];
  let paragraph: string[] = [];
  let list: string[] = [];

  const flush = () => {
    if (paragraph.length > 0) {
      const key = `p${blocks.length}`;
      blocks.push(<p key={key}>{inline(paragraph.join(' '), key)}</p>);
      paragraph = [];
    }
    if (list.length > 0) {
      const key = `u${blocks.length}`;
      blocks.push(
        <ul key={key}>
          {list.map((item, i) => (
            <li key={`${key}-${i}`}>{inline(item, `${key}-${i}`)}</li>
          ))}
        </ul>,
      );
      list = [];
    }
  };

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]!;
    const trimmed = line.trim();
    if (trimmed === '') {
      flush();
      continue;
    }
    const heading = /^(#{1,4})\s+(.*)$/.exec(trimmed);
    if (heading) {
      flush();
      const level = heading[1]!.length;
      const key = `h${blocks.length}`;
      const content = inline(heading[2]!, key);
      blocks.push(
        level <= 1 ? <h2 key={key}>{content}</h2> : level === 2 ? <h3 key={key}>{content}</h3> : <h4 key={key}>{content}</h4>,
      );
      continue;
    }
    if (trimmed.startsWith('- ')) {
      if (paragraph.length > 0) flush();
      list.push(trimmed.slice(2));
      continue;
    }
    // A table: header row, a line of dashes, then rows.
    if (trimmed.startsWith('|') && (lines[i + 1] ?? '').trim().startsWith('|-')) {
      flush();
      const head = cells(trimmed);
      const rows: string[][] = [];
      i += 2;
      while (i < lines.length && lines[i]!.trim().startsWith('|')) {
        rows.push(cells(lines[i]!));
        i++;
      }
      i--;
      const key = `t${blocks.length}`;
      blocks.push(
        <div className="md-table" key={key}>
          <table>
            <thead>
              <tr>
                {head.map((c, n) => (
                  <th key={n}>{inline(c, `${key}-h${n}`)}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {rows.map((row, r) => (
                <tr key={r}>
                  {row.map((c, n) => (
                    <td key={n}>{inline(c, `${key}-${r}-${n}`)}</td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>,
      );
      continue;
    }
    if (trimmed === '---') {
      flush();
      blocks.push(<hr key={`r${blocks.length}`} />);
      continue;
    }
    if (list.length > 0 && line.startsWith('  ')) {
      list[list.length - 1] += ` ${trimmed}`;
      continue;
    }
    if (list.length > 0) flush();
    paragraph.push(trimmed);
  }
  flush();
  return <div className="md">{blocks}</div>;
}
