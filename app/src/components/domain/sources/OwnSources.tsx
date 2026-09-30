import { Trans, useLingui } from "@lingui/react/macro";
import { CloudArrowDownIcon, PlusIcon } from "@phosphor-icons/react";
import { InfoHeading, List, ListRow } from "../../ui";
import type { CustomSource } from "../../../lib/types";

/** Feeds the user described: listed with what they answer and where they are asked. */
export function OwnSources({
  sources,
  busy,
  onEdit,
}: {
  sources: CustomSource[];
  busy: boolean;
  onEdit: (source: CustomSource | "new") => void;
}) {
  const { t } = useLingui();
  return (
    <>
      <div className="inline">
        <InfoHeading
          title={t`Your own sources`}
          info={t`Any JSON or CSV price endpoint: its address, and where the dates and closes sit in the answer.`}
        />
        <span className="spacer" />
        <button
          type="button"
          className="btn btn--sm btn--ghost"
          disabled={busy}
          onClick={() => onEdit("new")}
        >
          <PlusIcon /> <Trans>Add source</Trans>
        </button>
      </div>
      {sources.length > 0 && (
        <List variant="cards">
          {sources.map((source) => (
            <ListRow
              key={source.id}
              box
              lead={<CloudArrowDownIcon />}
              title={source.label}
              sub={`${source.role === "fx" ? t`exchange rates` : t`quotes`} · ${source.url}`}
              end={
                <button type="button" className="btn btn--sm btn--ghost" onClick={() => onEdit(source)}>
                  <Trans>Edit</Trans>
                </button>
              }
            />
          ))}
        </List>
      )}
    </>
  );
}
