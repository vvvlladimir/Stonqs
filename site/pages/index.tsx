import Head from "next/head";
import { Features } from "@/components/Features";
import { Download, Faq, Footer } from "@/components/Get";
import { Header, Hero } from "@/components/Hero";
import { Brokers, Chart, Returns } from "@/components/Story";
import { Assistant, Privacy } from "@/components/Trust";

const TITLE = "Stonqs: a private portfolio tracker for your desktop";
const DESCRIPTION =
  "Free, open-source portfolio tracker. True time-weighted and money-weighted returns, multi-currency accounts, broker CSV import. Runs on your computer, no account.";

export default function Home() {
  return (
    <>
      <Head>
        <title>{TITLE}</title>
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <meta name="description" content={DESCRIPTION} />
        <link rel="canonical" href="https://stonqs.app/" />
        <meta property="og:type" content="website" />
        <meta property="og:url" content="https://stonqs.app/" />
        <meta property="og:title" content="Stonqs: know how your investments are really doing" />
        <meta
          property="og:description"
          content="A private portfolio tracker with true returns, realised gains and multi-currency accounts. Free and open source."
        />
        <meta property="og:image" content="https://stonqs.app/assets/og.jpg" />
        <meta name="twitter:card" content="summary_large_image" />
      </Head>
      <Header />
      <main>
        <Hero />
        <Brokers />
        <Returns />
        <Chart />
        <Features />
        <Assistant />
        <Privacy />
        <Faq />
        <Download />
      </main>
      <Footer />
    </>
  );
}
