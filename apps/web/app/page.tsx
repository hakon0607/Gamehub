export default function Home() {
  return (
    <main style={{ maxWidth: 640, margin: '0 auto', padding: '80px 24px' }}>
      <h1 style={{ fontSize: 40, marginBottom: 8 }}>GameHub</h1>
      <p style={{ color: '#9aa1b4', lineHeight: 1.7 }}>
        Every PC game you own — Steam, Epic, Xbox, EA, Ubisoft, Battle.net, GOG and Riot — in one
        library that finds new installs by itself.
      </p>
      <p style={{ color: '#9aa1b4', lineHeight: 1.7 }}>
        The desktop app works entirely offline. This site only exists for optional account sync and the game
        assistant.
      </p>
      <a
        href="https://github.com/OWNER/gamehub/releases/latest"
        style={{
          display: 'inline-block',
          marginTop: 16,
          padding: '10px 20px',
          borderRadius: 8,
          background: '#6d7cff',
          color: '#fff',
          textDecoration: 'none',
        }}
      >
        Download for Windows
      </a>
    </main>
  );
}
