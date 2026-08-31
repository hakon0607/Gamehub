/**
 * Hands the browser a short-lived token so it can upload straight to Blob.
 *
 * The file never passes through this function, and that is the point: a Vercel
 * function refuses a request body over about 4.5 MB, so posting a 90 MB
 * installer to the server would fail every time. The browser uploads directly
 * and only the permission to do so is issued here — after the session cookie
 * has been checked.
 */
import { handleUpload, type HandleUploadBody } from '@vercel/blob/client';
import { NextResponse } from 'next/server';
import { cookies } from 'next/headers';
import { readSession, SESSION_COOKIE } from '@/lib/auth';

const MAX_BYTES = 500 * 1024 * 1024;

export async function POST(request: Request): Promise<NextResponse> {
  const body = (await request.json()) as HandleUploadBody;

  try {
    const result = await handleUpload({
      body,
      request,
      onBeforeGenerateToken: async (pathname) => {
        // Checked here, not in the page: this is the only place that actually
        // decides whether an upload may happen.
        const store = await cookies();
        const session = readSession(store.get(SESSION_COOKIE)?.value);
        if (!session) throw new Error('Du er ikke logget inn.');

        if (!/\.(exe|msi|zip)$/i.test(pathname)) {
          throw new Error('Bare .exe, .msi og .zip kan lastes opp.');
        }

        return {
          allowedContentTypes: [
            'application/octet-stream',
            'application/x-msdownload',
            'application/x-msdos-program',
            'application/vnd.microsoft.portable-executable',
            'application/zip',
          ],
          maximumSizeInBytes: MAX_BYTES,
          addRandomSuffix: true,
        };
      },
      onUploadCompleted: async ({ blob }) => {
        // Vercel calls this after a successful upload in production. The
        // release list is not written here: this callback cannot reach a
        // developer's machine, so relying on it would make local testing
        // behave differently from the deployed site. The browser posts the
        // details to /api/releases instead, which works in both.
        console.log('uploaded', blob.pathname);
      },
    });

    return NextResponse.json(result);
  } catch (error) {
    return NextResponse.json(
      { error: error instanceof Error ? error.message : 'Opplastingen mislyktes.' },
      { status: 400 },
    );
  }
}
