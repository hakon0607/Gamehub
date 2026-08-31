/**
 * Who is allowed to upload.
 *
 * The check is on the server and the proof is a signed cookie. A password
 * compared in the browser would be no protection at all: anyone can edit
 * JavaScript, and the upload API is reachable directly regardless of what the
 * page does. So every route that changes anything verifies the signature here.
 *
 * The signing secret comes from AUTH_SECRET. If that is not set the secret is
 * derived from the password, which still keeps cookies unforgeable by outsiders
 * but means changing the password logs everyone out — an acceptable trade for
 * not silently running with a blank secret.
 */
import { createHmac, timingSafeEqual, randomBytes } from 'node:crypto';

export const SESSION_COOKIE = 'gamehub_admin';
const MAX_AGE_SECONDS = 60 * 60 * 8;

export const DEFAULT_USERNAME = 'admin';
export const DEFAULT_PASSWORD = 'admin123';

export function adminUsername(): string {
  return process.env.ADMIN_USERNAME || DEFAULT_USERNAME;
}

export function adminPassword(): string {
  return process.env.ADMIN_PASSWORD || DEFAULT_PASSWORD;
}

/**
 * True while the site is still using the password that shipped with it.
 * The admin page shows a warning when this is the case — a default password on
 * a public URL is guessable by anyone, and whoever guesses it can publish a
 * .exe that other people will download and run.
 */
export function usingDefaultPassword(): boolean {
  return adminPassword() === DEFAULT_PASSWORD;
}

function secret(): string {
  return process.env.AUTH_SECRET || `derived:${adminUsername()}:${adminPassword()}`;
}

function sign(payload: string, key: string): string {
  return createHmac('sha256', key).update(payload).digest('base64url');
}

/** Constant-time compare that cannot throw on a length mismatch. */
export function safeEqual(a: string, b: string): boolean {
  const left = Buffer.from(String(a));
  const right = Buffer.from(String(b));
  if (left.length !== right.length) {
    // Still do a comparison so the timing does not reveal the length.
    timingSafeEqual(left, left);
    return false;
  }
  return timingSafeEqual(left, right);
}

export function checkCredentials(username: string, password: string): boolean {
  // Both are checked even when the first fails, so the timing says nothing
  // about which half was wrong.
  const userOk = safeEqual(username ?? '', adminUsername());
  const passOk = safeEqual(password ?? '', adminPassword());
  return userOk && passOk;
}

export interface SessionPayload {
  u: string;
  exp: number;
}

export function createSession(username: string, key = secret(), now = Date.now()): string {
  const payload: SessionPayload = {
    u: username,
    exp: Math.floor(now / 1000) + MAX_AGE_SECONDS,
  };
  const encoded = Buffer.from(JSON.stringify(payload)).toString('base64url');
  return `${encoded}.${sign(encoded, key)}`;
}

/** Returns the payload when the token is genuine and unexpired, else null. */
export function readSession(
  token: string | undefined,
  key = secret(),
  now = Date.now(),
): SessionPayload | null {
  if (!token) return null;
  const parts = token.split('.');
  if (parts.length !== 2) return null;

  const [encoded, signature] = parts;
  if (!safeEqual(signature, sign(encoded, key))) return null;

  try {
    const payload = JSON.parse(Buffer.from(encoded, 'base64url').toString('utf8')) as SessionPayload;
    if (typeof payload?.exp !== 'number' || payload.exp * 1000 < now) return null;
    return payload;
  } catch {
    return null;
  }
}

export function sessionCookieOptions() {
  return {
    httpOnly: true,
    sameSite: 'lax' as const,
    secure: process.env.NODE_ENV === 'production',
    path: '/',
    maxAge: MAX_AGE_SECONDS,
  };
}

/** Suggests a strong password for the admin page to offer. */
export function suggestPassword(): string {
  return randomBytes(12).toString('base64url');
}
