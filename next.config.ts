import type { NextConfig } from 'next';

const config: NextConfig = {
  // The release list changes when a human uploads one, and must be visible
  // immediately afterwards, so pages read it at request time.
  reactStrictMode: true,
};

export default config;
