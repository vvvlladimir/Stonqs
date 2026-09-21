import { Plural, Trans, useLingui } from "@lingui/react/macro";
import { useMemo, useState } from "react";
import { Badge, List, ListRow, Panel } from "../../components/ui";
import type { ImportProblem, ProblemCode } from "../../lib/types";
import { PROBLEM_LABELS, problemDetail } from "./labels";

/**
 * Parser notices grouped by code: a broken column complains on every row, and a
 * hundred identical lines say no more than one line plus a count. Opening a group
 * shows example rows, which is what makes the complaint fixable.
 */
export function ProblemList({
  problems,
  title,
  limit = 5,
}: {
  problems: ImportProblem[];
  title?: string;
  limit?: number;
}) {
  const { t, i18n } = useLingui();
  const [open, setOpen] = useState<ProblemCode | null>(null);

  const groups = useMemo(() => {
    const by = new Map<ProblemCode, ImportProblem[]>();
    for (const p of problems) {
      const list = by.get(p.code);
      if (list) list.push(p);
      else by.set(p.code, [p]);
    }
    return [...by.entries()].sort((a, b) => b[1].length - a[1].length);
  }, [problems]);

  if (groups.length === 0) return null;
  return (
    <Panel
      title={title ?? t`Parser notices`}
      note={<Plural value={groups.length} one="# kind" other="# kinds" />}
    >
      <List>
        {groups.map(([code, list]) => (
          <ListRow
            key={code}
            wrap
            on={open === code}
            onClick={() => setOpen(open === code ? null : code)}
            title={PROBLEM_LABELS[code] ? i18n._(PROBLEM_LABELS[code]) : code}
            sub={open === code ? t`hide rows` : t`show rows`}
            value={<Badge tone={list[0].severity === "ERROR" ? "out" : "warn"}>{list.length}</Badge>}
            foot={
              open === code ? (
                <>
                  {list.slice(0, limit).map((p, i) => (
                    <span key={i}>
                      {p.row === null ? "" : t`row ${p.row}: `}
                      {problemDetail(i18n, p)}
                    </span>
                  ))}
                  {list.length > limit && (
                    <span className="dim">
                      <Trans>…and {list.length - limit} more</Trans>
                    </span>
                  )}
                </>
              ) : undefined
            }
          />
        ))}
      </List>
    </Panel>
  );
}
