import { useState } from "react";
import { useLingui } from "@lingui/react/macro";
import { Page } from "../../components/Page";
import { Tabs } from "../../components/ui";
import type { AppStatus } from "../../lib/types";
import { AccountsPanel } from "./AccountsPanel";
import { AiPanel } from "./AiPanel";
import { AppearancePanel } from "./AppearancePanel";
import { AttributesPanel } from "./AttributesPanel";
import { CATEGORIES, type CategoryId } from "./model";
import { DataPanel } from "./DataPanel";
import { InflationPanel } from "./InflationPanel";
import { MarketDataPanel } from "./MarketDataPanel";
import { PortfolioPanel } from "./PortfolioPanel";
import { ProfilesPanel } from "./ProfilesPanel";

export function Settings({ status }: { status: AppStatus }) {
  const { t, i18n } = useLingui();
  const [category, setCategory] = useState<CategoryId>("portfolio");

  return (
    <Page archetype="form" title={t`Settings`}>
      <Tabs
        label={t`Settings sections`}
        value={category}
        onChange={setCategory}
        items={CATEGORIES.map(({ id, label, icon: Icon }) => ({
          id,
          label: i18n._(label),
          icon: <Icon />,
        }))}
      >
        {category === "portfolio" && (
          <>
            <PortfolioPanel />
            <InflationPanel />
          </>
        )}
        {category === "accounts" && <AccountsPanel />}
        {category === "attributes" && <AttributesPanel />}
        {category === "market" && <MarketDataPanel />}
        {category === "ai" && <AiPanel />}
        {category === "appearance" && <AppearancePanel />}
        {category === "profiles" && <ProfilesPanel />}
        {category === "data" && <DataPanel status={status} />}
      </Tabs>
    </Page>
  );
}
