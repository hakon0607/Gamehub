import { NextResponse } from 'next/server';
import { supabaseForRequest } from '@/lib/supabase';

/**
 * Cloud sync. GET returns the stored state; POST replaces it, refusing the
 * write when another device has already pushed something newer.
 *
 * What is accepted is deliberately narrow: no paths, no executables, no
 * launcher credentials. Anything else in the body is dropped before it reaches
 * the database.
 */
export async function GET() {
  const supabase = await supabaseForRequest();
  const { data: { user } } = await supabase.auth.getUser();
  if (!user) return NextResponse.json({ error: 'Not signed in' }, { status: 401 });

  const { data, error } = await supabase.from('sync_state').select('*').eq('user_id', user.id).maybeSingle();
  if (error) return NextResponse.json({ error: error.message }, { status: 500 });
  return NextResponse.json(data ?? { revision: 0 });
}

interface PushBody {
  payload?: Record<string, unknown>;
  baseRevision?: number;
}

/** Only these keys are stored. Everything else in the body is ignored. */
const ALLOWED = ['favorites', 'hidden', 'tags', 'playHistory', 'minecraftProfiles', 'settings'] as const;

export async function POST(request: Request) {
  const supabase = await supabaseForRequest();
  const { data: { user } } = await supabase.auth.getUser();
  if (!user) return NextResponse.json({ error: 'Not signed in' }, { status: 401 });

  let body: PushBody;
  try {
    body = (await request.json()) as PushBody;
  } catch {
    return NextResponse.json({ error: 'Malformed request' }, { status: 400 });
  }

  const source = body.payload ?? {};
  const payload: Record<string, unknown> = {};
  for (const key of ALLOWED) {
    if (key in source) payload[key] = source[key];
  }

  const { data, error } = await supabase.rpc('push_sync_state', {
    payload,
    base_revision: Number(body.baseRevision ?? 0),
  });

  if (error) {
    const conflict = error.message.includes('sync_conflict');
    return NextResponse.json(
      { error: conflict ? 'Another device synced first — pull before pushing.' : error.message },
      { status: conflict ? 409 : 500 },
    );
  }
  return NextResponse.json(data);
}
