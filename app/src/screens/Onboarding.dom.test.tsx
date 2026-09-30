/** A new profile: the portfolio form, then the sources question on the same gate. Nothing about
 *  the portfolio is written until the second step is left, and the answer is written first. */
import { beforeEach, describe, expect, it } from "vitest";
import userEvent from "@testing-library/user-event";
import { screen } from "@testing-library/react";
import { mockHost, renderScreen, type Asked } from "../test/host";
import { Onboarding } from "./Onboarding";
import type { AppStatus, MarketSourceRow } from "../lib/types";

const status: AppStatus = {
  portfolio_name: "Portfolio",
  base_currency: "EUR",
  inception: null,
  account_count: 0,
  db_path: "/tmp/stonqs.db",
  dev_build: false,
  system_locale: null,
};

const source = (id: string, capability: MarketSourceRow["capabilities"][number]): MarketSourceRow => ({
  id,
  site: `https://${id}.example`,
  capabilities: [capability],
  key: "none",
  has_key: false,
  on_by_default: false,
  wanted: false,
  active: false,
});

let asked: Asked;

beforeEach(() => {
  const rows = [source("yahoo", "quotes"), source("ecb", "fx_rates")];
  asked = mockHost({
    profiles_list: () => ({ profiles: [], open: "p1", locked: false, remembered: false }),
    settings_get: () => ({ sources_configured: false, market_sources: {} }),
    market_sources_list: () => rows,
    market_custom_list: () => [],
    securities_list: () => [],
    market_source_switch: ({ source: id, on }) => {
      const row = rows.find((r) => r.id === id);
      if (row) row.wanted = on as boolean;
      return null;
    },
    market_sources_confirm: () => null,
    setup_portfolio: () => ({}),
    demo_seed: () => null,
  });
});

const commands = () => asked.map((a) => a.command);

async function throughTheForm(user: ReturnType<typeof userEvent.setup>) {
  renderScreen(<Onboarding status={status} />);
  await user.type(screen.getByLabelText(/cash account/i), "Main");
  await user.click(screen.getByRole("button", { name: /^continue$/i }));
  await screen.findByRole("heading", { name: /where data comes from/i });
}

describe("onboarding", () => {
  it("asks for sources before the portfolio is written, and writes the answer first", async () => {
    const user = userEvent.setup();
    await throughTheForm(user);
    expect(commands()).not.toContain("setup_portfolio");

    await user.click(await screen.findByRole("button", { name: /select all and continue/i }));
    await expect.poll(() => commands()).toContain("setup_portfolio");

    const order = commands().filter((c) => c !== "settings_get" && !c.endsWith("_list"));
    expect(order).toEqual([
      "market_source_switch",
      "market_source_switch",
      "market_sources_confirm",
      "setup_portfolio",
    ]);
    const setup = asked.find((a) => a.command === "setup_portfolio");
    expect(setup?.args.input).toMatchObject({ account_name: "Main", base_currency: "EUR" });
  });

  it("writes the portfolio without an answer when the question is left for later", async () => {
    const user = userEvent.setup();
    await throughTheForm(user);
    await user.click(screen.getByRole("button", { name: /decide later/i }));
    await expect.poll(() => commands()).toContain("setup_portfolio");
    expect(commands()).not.toContain("market_sources_confirm");
  });

  it("goes back to the form with what was typed", async () => {
    const user = userEvent.setup();
    await throughTheForm(user);
    await user.click(screen.getByRole("button", { name: /^back$/i }));
    const field = await screen.findByLabelText<HTMLInputElement>(/cash account/i);
    expect(field.value).toBe("Main");
  });

  it("asks the same question on the way to the demo portfolio", async () => {
    const user = userEvent.setup();
    renderScreen(<Onboarding status={status} />);
    await user.click(screen.getByRole("button", { name: /try with a demo portfolio/i }));
    await screen.findByRole("heading", { name: /where data comes from/i });
    expect(commands()).not.toContain("demo_seed");
    await user.click(screen.getByRole("button", { name: /decide later/i }));
    await expect.poll(() => commands()).toContain("demo_seed");
  });
});
