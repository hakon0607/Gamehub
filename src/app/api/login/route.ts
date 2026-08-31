import { NextResponse } from 'next/server';
import { checkCredentials, createSession, SESSION_COOKIE, sessionCookieOptions } from '@/lib/auth';

export async function POST(request: Request) {
  let username = '';
  let password = '';
  try {
    const body = await request.json();
    username = String(body?.username ?? '');
    password = String(body?.password ?? '');
  } catch {
    return NextResponse.json({ error: 'Ugyldig forespørsel.' }, { status: 400 });
  }

  if (!checkCredentials(username, password)) {
    // One message for both wrong username and wrong password, so the form
    // never confirms that a username exists.
    return NextResponse.json({ error: 'Feil brukernavn eller passord.' }, { status: 401 });
  }

  const response = NextResponse.json({ ok: true });
  response.cookies.set(SESSION_COOKIE, createSession(username), sessionCookieOptions());
  return response;
}
