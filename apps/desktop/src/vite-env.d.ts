/// <reference types="vite/client" />

/** Markdown imported as text — the legal documents ship inside the bundle. */
declare module '*.md?raw' {
  const content: string;
  export default content;
}
