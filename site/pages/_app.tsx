import type { AppProps } from "next/app";
import localFont from "next/font/local";
import { MotionConfig } from "motion/react";
import "@/styles/globals.css";

const geist = localFont({
  src: "../public/assets/fonts/geist-latin-wght-normal.woff2",
  weight: "100 900",
  variable: "--font-geist",
  display: "swap",
});
const geistMono = localFont({
  src: "../public/assets/fonts/geist-mono-latin-wght-normal.woff2",
  weight: "100 900",
  variable: "--font-geist-mono",
  display: "swap",
});

export default function App({ Component, pageProps }: AppProps) {
  return (
    <MotionConfig reducedMotion="user">
      <div className={`${geist.variable} ${geistMono.variable} font-sans`}>
        <Component {...pageProps} />
      </div>
    </MotionConfig>
  );
}
