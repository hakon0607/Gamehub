'use client';

import { useState } from 'react';

export function LoginForm() {
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);

  return (
    <div className="wrap" style={{ maxWidth: 420, paddingTop: 80, paddingBottom: 60 }}>
      <h2 className="section">Logg inn</h2>
      <p className="section-note">Bare for deg som legger ut nye versjoner.</p>

      <form
        className="panel"
        onSubmit={async (event) => {
          event.preventDefault();
          setBusy(true);
          setError('');
          try {
            const response = await fetch('/api/login', {
              method: 'POST',
              headers: { 'content-type': 'application/json' },
              body: JSON.stringify({ username, password }),
            });
            if (!response.ok) {
              const body = await response.json().catch(() => ({}));
              setError(body.error ?? 'Innloggingen mislyktes.');
              return;
            }
            // A full reload, so the server renders the dashboard with a fresh
            // session rather than the client guessing at the state.
            window.location.reload();
          } catch {
            setError('Fikk ikke kontakt med serveren.');
          } finally {
            setBusy(false);
          }
        }}
      >
        {error && <div className="notice notice-bad">{error}</div>}

        <div className="field">
          <label htmlFor="u">Brukernavn</label>
          <input
            id="u"
            type="text"
            autoComplete="username"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            required
          />
        </div>

        <div className="field">
          <label htmlFor="p">Passord</label>
          <input
            id="p"
            type="password"
            autoComplete="current-password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
          />
        </div>

        <button className="btn btn-accent" type="submit" disabled={busy}>
          {busy && <span className="spinner" aria-hidden="true" />}
          {busy ? 'Logger inn …' : 'Logg inn'}
        </button>
      </form>
    </div>
  );
}
