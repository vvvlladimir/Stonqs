import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import {
  BankIcon,
  CaretUpDownIcon,
  CertificateIcon,
  DatabaseIcon,
  FolderIcon,
  type Icon,
} from "@phosphor-icons/react";
import type { I18n } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { Plural, Trans, useLingui } from "@lingui/react/macro";
import { api } from "../../lib/api";
import { affects, useInvalidate, useScope } from "../../lib/queries";
import { ErrorText, ListRow, Modal } from "../ui";
import type { DataScope, ScopeOption } from "../../lib/types";

/** Global data scope selector shared by all screens. */
const ICONS: Record<string, Icon> = {
  portfolio: DatabaseIcon,
  group: FolderIcon,
  account: BankIcon,
  account_with_cash: CertificateIcon,
};

export function ScopePicker({ variant }: { variant: "nav" | "dock" }) {
  const { t, i18n } = useLingui();
  const invalidate = useInvalidate();
  const [open, setOpen] = useState(false);
  const scope = useScope();

  const set = useMutation({
    mutationFn: (value: DataScope) => api.scopeSet(value),
    onSuccess: () => {
      setOpen(false);
      invalidate(...affects.scope);
    },
  });

  if (!scope.data) return null;

  // Scope kind and ID together form a stable key; portfolio has no ID.
  const keyOf = (o: Pick<ScopeOption, "kind" | "id">) => `${o.kind}:${o.id ?? ""}`;
  const current = scope.data.options.find((o) => keyOf(o) === keyOf(scope.data.scope));
  const CurrentIcon = ICONS[scope.data.scope.kind] ?? DatabaseIcon;

  const label = current ? scopeLabel(i18n, current) : t`Whole portfolio`;
  const button =
    variant === "nav" ? (
      <button
        type="button"
        className="scope scope--row"
        data-tip={label}
        onClick={() => setOpen(true)}
        disabled={set.isPending}
      >
        <CurrentIcon className="scope__icon" />
        <span className="scope__label">
          {current?.kind === "PORTFOLIO" || !current ? t`Whole portfolio` : label}
        </span>
        <CaretUpDownIcon className="scope__caret" />
      </button>
    ) : (
      <button type="button" className="scope" onClick={() => setOpen(true)} disabled={set.isPending}>
        <CurrentIcon className="scope__icon" />
        <span className="min0">
          <span className="scope__label">{label}</span>
          <span className="scope__sub">
            <Subtitle option={current} />
          </span>
        </span>
        <CaretUpDownIcon className="scope__caret" />
      </button>
    );

  return (
    <>
      {button}

      {open && (
        <Modal title={t`Data source`} onClose={() => setOpen(false)}>
          <div className="smenu">
            {scope.data.options.map((option) => {
              const OptionIcon = ICONS[option.kind] ?? DatabaseIcon;
              const active = keyOf(option) === keyOf(scope.data.scope);
              return (
                <ListRow
                  key={keyOf(option)}
                  lead={<OptionIcon />}
                  title={scopeLabel(i18n, option)}
                  sub={<Subtitle option={option} />}
                  on={active}
                  onClick={() => set.mutate({ kind: option.kind, id: option.id })}
                />
              );
            })}
          </div>
          <ErrorText error={set.error} />
        </Modal>
      )}
    </>
  );
}

/**
 * The one place the scope's wording is written. The host sends names and a kind, so
 * "Securities · Depot + Cash" is composed here, in the active language.
 */
export function scopeLabel(i18n: I18n, option: ScopeOption): string {
  switch (option.kind) {
    case "PORTFOLIO":
      return i18n._(msg`Whole portfolio · ${option.name}`);
    case "GROUP":
      return i18n._(msg`Group · ${option.name}`);
    case "ACCOUNT":
      return option.account_kind === "SECURITIES"
        ? i18n._(msg`Securities · ${option.name}`)
        : i18n._(msg`Cash · ${option.name}`);
    case "ACCOUNT_WITH_CASH":
      return i18n._(msg`Securities · ${option.name} + ${option.cash_name ?? ""}`);
  }
}

function Subtitle({ option }: { option?: ScopeOption }) {
  if (!option) return null;
  if (option.account_count === 0) return <Trans>empty</Trans>;
  return (
    <Plural
      value={option.account_count}
      one="# account"
      few="# accounts"
      many="# accounts"
      other="# accounts"
    />
  );
}
