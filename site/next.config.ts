import path from "node:path";
import type { NextConfig } from "next";

// A static export for Cloudflare Pages. The Pages router, not the App router: App router pages
// carry inline <script> payloads that the CSP in public/_headers (script-src 'self') would block.
const config: NextConfig = {
  output: "export",
  images: { unoptimized: true },
  reactStrictMode: true,
  turbopack: { root: path.resolve(".") },
};

export default config;
