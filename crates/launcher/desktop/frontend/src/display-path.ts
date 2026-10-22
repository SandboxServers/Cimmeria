// Windows writes long and UNC paths with a `\\?\` prefix. The launcher shows the plain form to the player.
const extendedUnc=/^\\\\\?\\UNC\\/;
const extendedDrive=/^\\\\\?\\[A-Za-z]:\\/;

/** A folder path as the player should read it: `\\?\UNC\server\share` is `\\server\share`, `\\?\C:\x` is `C:\x`. Anything else is returned unchanged. */
export function displayPath(p: string): string {
  if (extendedUnc.test(p)) return `\\\\${p.slice(8)}`;
  if (extendedDrive.test(p)) return p.slice(4);
  return p;
}
