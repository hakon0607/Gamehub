import { NextResponse } from 'next/server';
import { cookies } from 'next/headers';
import { readSession, SESSION_COOKIE, usingDefaultPassword } from '@/lib/auth';
import { blobConfigured } from '@/lib/releases';

export async function GET() {
  const store = await cookies();
  const session = readSession(store.get(SESSION_COOKIE)?.value);
  return NextResponse.json({
    signedIn: session !== null,
    username: session?.u ?? null,
    defaultPassword: usingDefaultPassword(),
    blobConfigured: blobConfigured(),
  });
}
