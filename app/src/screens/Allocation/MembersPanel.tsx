import { Trans, useLingui } from "@lingui/react/macro";
import { ProhibitIcon } from "@phosphor-icons/react";
import type { UseQueryResult } from "@tanstack/react-query";
import { Instrument } from "../../components/domain/Instrument";
import { Async, DataTable, Empty, Money, Panel, Percent, Swatch } from "../../components/ui";
import { toNumber } from "../../lib/format";
import { slotOf } from "../../lib/taxonomy";
import type { NodeMember, TaxonomyData } from "../../lib/types";
import { assignedShare, nodeName } from "./model";

/** Securities and account cash of the current level with their shares. */
export function MembersPanel({
  taxonomy,
  members,
  levelKey,
  currency,
  onOpenCard,
  onAssign,
}: {
  taxonomy: TaxonomyData;
  members: UseQueryResult<NodeMember[]>;
  levelKey: string | null;
  currency: string;
  onOpenCard: (subjectId: string) => void;
  onAssign: (subjectId: string) => void;
}) {
  const { t } = useLingui();
  return (
    <Panel
      title={t`Split`}
      info={
        levelKey
          ? t`Click a row to open the instrument, or its categories to change how it is split.`
          : t`Instruments and cash with their shares; a broad fund can sit in several categories at once.`
      }
      table
    >
      <Async
        query={members}
        empty={
          <Empty title={t`Nothing here`}>
            <Trans>No instrument of the current data source falls under this level.</Trans>
          </Empty>
        }
      >
        {(rows) => (
          <DataTable
            className="assign"
            fixed
            card={false}
            rows={rows}
            rowKey={(member) => member.subject_id}
            rowProps={(member) => ({ className: member.excluded ? "is-off" : undefined })}
            columns={[
              {
                key: "subject",
                sort: (member) => member.name || member.symbol,
                header: t`Instrument`,
                align: "left",
                width: "44%",
                cell: (member) => (
                  <button
                    type="button"
                    className="linkrow"
                    onClick={() =>
                      member.kind === "SECURITY" && !member.excluded
                        ? onOpenCard(member.subject_id)
                        : onAssign(member.subject_id)
                    }
                  >
                    <Instrument symbol={member.symbol} name={member.name} />
                  </button>
                ),
              },
              {
                key: "nodes",
                sort: (member) => (member.excluded ? 1 : 0),
                header: t`Categories`,
                align: "left",
                width: "40%",
                cell: (member) => (
                  <button
                    type="button"
                    className="linkrow"
                    data-tip={t`Click to change the shares`}
                    onClick={() => onAssign(member.subject_id)}
                  >
                    {member.excluded ? (
                      <span className="pill pill--empty">
                        <ProhibitIcon /> <Trans>disabled</Trans>
                      </span>
                    ) : (
                      <Pills taxonomy={taxonomy} subjectId={member.subject_id} />
                    )}
                  </button>
                ),
              },
              {
                key: "value",
                sort: (member) => toNumber(member.value_base),
                header: levelKey ? t`Here` : t`Value`,
                cell: (member) => (
                  <>
                    <Money value={member.value_base} currency={currency} />
                    {/* A security split across nodes shows what it is a part of. */}
                    {Number(member.assigned_share) < 0.9999 && (
                      <div className="sub">
                        <Trans>
                          of <Money value={member.subject_value_base} currency={currency} />
                        </Trans>
                      </div>
                    )}
                  </>
                ),
              },
            ]}
          />
        )}
      </Async>
    </Panel>
  );
}

function Pills({ taxonomy, subjectId }: { taxonomy: TaxonomyData; subjectId: string }) {
  const { t } = useLingui();
  const mine = taxonomy.classifications.filter((c) => c.subject_id === subjectId);
  const rest = 1 - assignedShare(taxonomy, subjectId);

  // A span, not a div: the cell that holds these pills is a button.
  return (
    <span className="pills">
      {mine.map((c) => {
        const node = taxonomy.nodes.find((n) => n.id === c.node_id);
        if (!node) return null;
        return (
          <span className="pill" key={c.node_id}>
            <Swatch slot={slotOf(taxonomy, node)} />
            {nodeName(taxonomy, node)}
            {Number(c.weight) < 0.9999 && <Percent value={c.weight} digits={2} dim />}
          </span>
        );
      })}
      {rest > 0.0001 && (
        <span className="pill pill--empty">
          {mine.length > 0 ? t`${Math.round(rest * 100)} % left` : t`not classified`}
        </span>
      )}
    </span>
  );
}
