export const metadata = {
  title: 'GameHub',
  description: 'Every PC game you own, in one place.',
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body style={{ margin: 0, background: '#0b0d12', color: '#e8eaf0', fontFamily: 'system-ui, sans-serif' }}>
        {children}
      </body>
    </html>
  );
}
