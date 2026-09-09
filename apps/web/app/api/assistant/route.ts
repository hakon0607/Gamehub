import { NextResponse } from 'next/server';

/**
 * The game assistant.
 *
 * It answers questions about the user's own library — "what should I play",
 * "something like Minecraft", "something for four people" — and nothing else.
 * The library is sent as names, hours and tags; no paths and no files.
 *
 * Any OpenAI-compatible provider works, so the free tiers (Gemini, Groq) are
 * usable without a card. The key lives on the server, never in the app bundle.
 */
const PROVIDERS: Record<string, { baseUrl: string; model: string }> = {
  gemini: {
    baseUrl: 'https://generativelanguage.googleapis.com/v1beta/openai',
    model: 'gemini-2.5-flash',
  },
  groq: { baseUrl: 'https://api.groq.com/openai/v1', model: 'llama-3.3-70b-versatile' },
  openrouter: { baseUrl: 'https://openrouter.ai/api/v1', model: 'meta-llama/llama-3.3-70b-instruct' },
  openai: { baseUrl: 'https://api.openai.com/v1', model: 'gpt-4o-mini' },
};

interface LibraryEntry {
  name: string;
  source: string;
  hours?: number;
  lastPlayed?: string | null;
  tags?: string[];
}

const SYSTEM = `You help someone choose what to play from the games they already own.
Recommend only games from the provided library. Be brief: name two or three, and say in
one line why each fits what they asked for. If nothing in the library fits, say so plainly
rather than inventing a game they do not have.`;

export async function POST(request: Request) {
  const providerName = process.env.AI_PROVIDER ?? 'gemini';
  const provider = PROVIDERS[providerName];
  const apiKey = process.env.AI_API_KEY;

  if (!provider || !apiKey) {
    return NextResponse.json(
      { error: 'The assistant is not configured on this server. Set AI_PROVIDER and AI_API_KEY.' },
      { status: 503 },
    );
  }

  let body: { question?: string; library?: LibraryEntry[] };
  try {
    body = await request.json();
  } catch {
    return NextResponse.json({ error: 'Malformed request' }, { status: 400 });
  }

  const question = (body.question ?? '').slice(0, 500);
  // Capped so a 900-game library cannot turn one question into a huge request.
  const library = (body.library ?? []).slice(0, 300).map((g) => ({
    name: g.name,
    source: g.source,
    hours: g.hours ? Math.round(g.hours) : undefined,
    lastPlayed: g.lastPlayed ?? undefined,
    tags: g.tags?.slice(0, 6),
  }));

  if (!question.trim()) return NextResponse.json({ error: 'Ask something first.' }, { status: 400 });

  const response = await fetch(`${provider.baseUrl}/chat/completions`, {
    method: 'POST',
    headers: { 'content-type': 'application/json', authorization: `Bearer ${apiKey}` },
    body: JSON.stringify({
      model: process.env.AI_MODEL ?? provider.model,
      messages: [
        { role: 'system', content: SYSTEM },
        { role: 'user', content: `My library:\n${JSON.stringify(library)}\n\nQuestion: ${question}` },
      ],
      temperature: 0.7,
      max_tokens: 400,
    }),
  });

  if (!response.ok) {
    const detail = await response.text();
    return NextResponse.json(
      { error: `${providerName} refused the request (${response.status}).`, detail: detail.slice(0, 400) },
      { status: 502 },
    );
  }

  const json = (await response.json()) as { choices?: { message?: { content?: string } }[] };
  return NextResponse.json({ answer: json.choices?.[0]?.message?.content ?? '' });
}
