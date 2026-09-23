import { plural } from "@lingui/core/macro";
import { useState } from "react";
import { Trans, useLingui } from "@lingui/react/macro";
import { ChartPieSliceIcon, WalletIcon } from "@phosphor-icons/react";
import { Banner, Choice, List, ListRow } from "../../components/ui";
import type { AccountKind, AccountRow, ImportMapping } from "../../lib/types";
import { accountLabel } from "./labels";

/**
 * Which account this export belongs to, asked as what the file *is* rather than as which of the
 * portfolio's accounts to pick. A broker statement and a bank statement are two different
 * things, and the cash leg of the first follows from the choice instead of being a second
 * question — so the step asks once and then only narrows.
 */
export function AccountTarget({
  accounts,
  mapping,
  onChange,
}: {
  accounts: AccountRow[];
  mapping: ImportMapping;
  onChange: (mapping: ImportMapping) => void;
}) {
  const { t, i18n } = useLingui();
  const target = accounts.find((a) => a.id === mapping.account_id) ?? null;
  // The card answers a question the mapping cannot always hold: with two accounts of that kind
  // there is no account to record yet, and reading the choice off `account_id` alone would
  // forget it the moment it was made. A recorded account still wins, so applying a layout —
  // which sets one directly — moves the cards with it.
  const [picked, setPicked] = useState<AccountKind | null>(null);
  const kind: AccountKind | null = target?.kind ?? picked;

  const of = (want: AccountKind) => accounts.filter((a) => a.kind === want);
  // A choice with one possible answer is not a question: picking the card picks the account.
  const pick = (want: AccountKind) => {
    const only = of(want);
    setPicked(want);
    onChange({ ...mapping, account_id: only.length === 1 ? only[0].id : null });
  };

  // A securities account never holds the money: its cash leg is the account it references.
  const cash =
    target?.kind === "SECURITIES"
      ? (accounts.find((a) => a.id === target.reference_account_id) ?? null)
      : target;
  const choices = kind ? of(kind) : [];

  return (
    <>
      <List variant="picks">
        <ListRow
          box
          wrap
          on={kind === "SECURITIES"}
          onClick={() => pick("SECURITIES")}
          lead={<ChartPieSliceIcon />}
          title={t`A broker statement`}
          sub={t`Purchases, sales, dividends. Instruments are recorded on the brokerage account, and the money moves on the cash account linked to it — no second choice to make.`}
          foot={plural(of("SECURITIES").length, { one: "# account", other: "# accounts" })}
        />
        <ListRow
          box
          wrap
          on={kind === "DEPOSIT"}
          onClick={() => pick("DEPOSIT")}
          lead={<WalletIcon />}
          title={t`A bank or wallet statement`}
          sub={t`Deposits, withdrawals, interest, fees. Everything lands on this one cash account; a row that carries an instrument does not belong in such a file.`}
          foot={plural(of("DEPOSIT").length, { one: "# account", other: "# accounts" })}
        />
      </List>

      {kind && choices.length === 0 && (
        <Banner tone="warn">
          {kind === "SECURITIES" ? (
            <Trans>
              The portfolio has no brokerage account yet. Create one on the "Accounts" screen — it needs a
              cash account to settle on — and come back to this step.
            </Trans>
          ) : (
            <Trans>The portfolio has no cash account yet. Create one on the "Accounts" screen.</Trans>
          )}
        </Banner>
      )}

      {/* Always on the step, inert until a card is chosen: a control that appears only once an
          answer has been given reads as a consequence of it rather than as the rest of it. */}
      <Choice
        wide
        disabled={kind === null}
        label={t`Which account`}
        placeholder={kind === null ? t`— choose above first —` : t`— choose the account —`}
        value={mapping.account_id ?? ""}
        onChange={(id) => onChange({ ...mapping, account_id: id || null })}
        options={choices.map((a) => ({ value: a.id, label: accountLabel(i18n, a) }))}
      />

      {target && (
        <List>
          <ListRow
            title={t`Rows land on`}
            sub={t`what the file's operations are recorded against`}
            value={accountLabel(i18n, target)}
          />
          <ListRow
            title={t`Money moves on`}
            sub={
              target.kind === "SECURITIES"
                ? t`the cash account this one settles through`
                : t`the same account`
            }
            value={cash ? accountLabel(i18n, cash) : "—"}
          />
        </List>
      )}
    </>
  );
}
