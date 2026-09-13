/**
 * Tests for the release guard. Plain node, no framework — this runs on the
 * GitHub runner before anything expensive starts, so a broken guard is caught
 * before a build rather than after a bad release.
 */
import { decide, parseVersion, compareVersions, highestReleasedVersion } from './release-guard.mjs';

let failures = 0;
const check = (name, condition) => {
  if (condition) console.log(`  ok  ${name}`);
  else { console.log(`FAIL  ${name}`); failures += 1; }
};

check('parses a plain version', JSON.stringify(parseVersion('0.2.1')) === '[0,2,1]');
check('rejects nonsense', parseVersion('v0.2') === null && parseVersion('') === null);
check('compares numerically not lexically', compareVersions([0,2,10], [0,2,9]) > 0);
check('finds the highest tag', JSON.stringify(highestReleasedVersion(['v0.1.0','v0.10.0','v0.2.0','main'])) === '[0,10,0]');

const fresh = decide({ version: '0.2.1', tags: ['v0.1.0', 'v0.2.0'] });
check('a new version is released', fresh.ok && fresh.tag === 'v0.2.1');

const duplicate = decide({ version: '0.2.0', tags: ['v0.2.0'] });
check('an existing tag is refused', !duplicate.ok);
check('the refusal names the file and the next number', duplicate.reason.includes('tauri.conf.json') && duplicate.reason.includes('0.2.1'));

const backwards = decide({ version: '0.1.9', tags: ['v0.2.0'] });
check('going backwards is refused', !backwards.ok && backwards.reason.includes('older'));

const forced = decide({ version: '0.2.0', tags: ['v0.2.0'], force: true });
check('the manual override still works', forced.ok);

const bad = decide({ version: '2.0', tags: [] });
check('a malformed version is refused clearly', !bad.ok && bad.reason.includes('three numbers'));

const first = decide({ version: '0.1.0', tags: [] });
check('the very first release goes through', first.ok);

console.log(failures === 0 ? '\nall guard tests passed' : `\n${failures} FAILED`);
process.exit(failures === 0 ? 0 : 1);
