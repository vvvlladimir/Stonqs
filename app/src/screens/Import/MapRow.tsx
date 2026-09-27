import { Plural, useLingui } from "@lingui/react/macro";
import type { ReactNode } from "react";
import { ListRow, Tag } from "../../components/ui";

/** One file value and the decision it waits for; every mapping list uses this row. */
export function MapRow({
  value,
  occurrences,
  done,
  sub,
  children,
}: {
  value: string;
  /** Rows behind the value; absent where counting them says nothing. */
  occurrences?: number;
  /** Undecided rows carry a warning tag; a row with nothing to decide omits it. */
  done?: boolean;
  sub?: ReactNode;
  children?: ReactNode;
}) {
  const { t } = useLingui();
  return (
    <ListRow
      title={<span className="mono">{value}</span>}
      sub={
        <span className="inline">
          {occurrences !== undefined && (
            <span>
              <Plural value={occurrences} one="# row" other="# rows" />
            </span>
          )}
          {sub}
          {done === false && <Tag warn>{t`not mapped`}</Tag>}
        </span>
      }
      end={children}
    />
  );
}
