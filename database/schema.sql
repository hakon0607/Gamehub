-- GameHub cloud sync.
--
-- What is here: the decisions a user made about their library. What is
-- deliberately not here: install paths, executables, launcher credentials and
-- game files. Sync exists so a new PC starts with your favourites and your
-- Minecraft profiles, not so anything of yours ends up on a server.
--
-- Run this once in the Supabase SQL editor (or against any Postgres).

create extension if not exists "pgcrypto";

create table if not exists public.profiles (
  id uuid primary key references auth.users (id) on delete cascade,
  display_name text,
  created_at timestamptz not null default now()
);

comment on table public.profiles is 'One row per signed-in user.';

create table if not exists public.devices (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references auth.users (id) on delete cascade,
  name text not null,
  last_seen_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

create index if not exists devices_user_idx on public.devices (user_id);

-- The synced state, one row per user. It is small and always written whole, so
-- a single JSONB column is both simpler and cheaper than a table per concept.
create table if not exists public.sync_state (
  user_id uuid primary key references auth.users (id) on delete cascade,
  favorites text[] not null default '{}',
  hidden text[] not null default '{}',
  tags jsonb not null default '{}'::jsonb,
  play_history jsonb not null default '[]'::jsonb,
  minecraft_profiles jsonb not null default '[]'::jsonb,
  settings jsonb not null default '{}'::jsonb,
  -- Bumped by the client on every push; a device with an older revision
  -- refetches instead of overwriting a newer state.
  revision bigint not null default 0,
  updated_at timestamptz not null default now()
);

comment on column public.sync_state.play_history is
  'Game id, last played and total seconds. No paths, ever.';

alter table public.profiles enable row level security;
alter table public.devices enable row level security;
alter table public.sync_state enable row level security;

-- Every policy is "your own row and nothing else". There is no shared or public
-- read anywhere in this schema.
drop policy if exists "own profile" on public.profiles;
create policy "own profile" on public.profiles
  for all using (auth.uid() = id) with check (auth.uid() = id);

drop policy if exists "own devices" on public.devices;
create policy "own devices" on public.devices
  for all using (auth.uid() = user_id) with check (auth.uid() = user_id);

drop policy if exists "own sync state" on public.sync_state;
create policy "own sync state" on public.sync_state
  for all using (auth.uid() = user_id) with check (auth.uid() = user_id);

-- Atomic push: refuses to overwrite a state that is newer than the one the
-- device last saw, so two PCs syncing at once cannot silently lose a change.
create or replace function public.push_sync_state(payload jsonb, base_revision bigint)
returns public.sync_state
language plpgsql
security invoker
set search_path = public, pg_temp
as $$
declare
  result public.sync_state;
begin
  insert into public.sync_state as s (
    user_id, favorites, hidden, tags, play_history, minecraft_profiles, settings, revision, updated_at
  )
  values (
    auth.uid(),
    coalesce(array(select jsonb_array_elements_text(payload -> 'favorites')), '{}'),
    coalesce(array(select jsonb_array_elements_text(payload -> 'hidden')), '{}'),
    coalesce(payload -> 'tags', '{}'::jsonb),
    coalesce(payload -> 'playHistory', '[]'::jsonb),
    coalesce(payload -> 'minecraftProfiles', '[]'::jsonb),
    coalesce(payload -> 'settings', '{}'::jsonb),
    base_revision + 1,
    now()
  )
  on conflict (user_id) do update
    set favorites = excluded.favorites,
        hidden = excluded.hidden,
        tags = excluded.tags,
        play_history = excluded.play_history,
        minecraft_profiles = excluded.minecraft_profiles,
        settings = excluded.settings,
        revision = excluded.revision,
        updated_at = now()
    where s.revision <= base_revision
  returning * into result;

  if result is null then
    raise exception 'sync_conflict' using hint = 'Another device has newer state; pull first.';
  end if;

  return result;
end;
$$;
