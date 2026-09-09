import { createServerClient, type CookieOptions } from '@supabase/ssr';
import { cookies } from 'next/headers';

/**
 * A Supabase client bound to the caller's session.
 *
 * Every table this app touches has row-level security, and this client carries
 * the user's own token — so a bug in a route handler cannot read another user's
 * row. The service-role key is never used here and must never reach the client
 * bundle.
 */
export async function supabaseForRequest() {
  const store = await cookies();
  return createServerClient(
    process.env.NEXT_PUBLIC_SUPABASE_URL!,
    process.env.NEXT_PUBLIC_SUPABASE_ANON_KEY!,
    {
      cookies: {
        getAll: () => store.getAll(),
        setAll: (list: { name: string; value: string; options: CookieOptions }[]) => {
          for (const { name, value, options } of list) store.set(name, value, options);
        },
      },
    },
  );
}
