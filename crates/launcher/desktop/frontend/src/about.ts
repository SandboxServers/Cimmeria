import type { NativeSnapshot } from "./contract";

type Build = NativeSnapshot["launcher"];

/** The About line in Settings (LX-17): the compiled version, and the release tag or that it has none. */
export function aboutText(build: Build | null): string {
  if (!build) return 'Reading launcher version…';
  return build.tag
    ? `Stargate Worlds Launcher ${build.version} · release ${build.tag}`
    : `Stargate Worlds Launcher ${build.version} · development build`;
}

/** The header badge: blank until the snapshot says which kind of build this is. */
export function buildBadge(build: Build | null): string {
  if (!build) return '';
  return build.tag ? `Version ${build.version}` : 'Development build';
}
