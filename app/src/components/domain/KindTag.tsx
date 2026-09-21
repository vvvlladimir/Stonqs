import { useLingui } from "@lingui/react/macro";
import { Badge, Tag, type BadgeTone } from "../ui";
import { transactionLabel } from "../../lib/kinds";
import type { TransactionKind } from "../../lib/types";

/**
 * The name of a transaction kind, wherever it is shown. A `tone` makes it a badge —
 * the journal colours the kind by the direction of its cash flow.
 */
export function KindTag({ kind, tone }: { kind: TransactionKind; tone?: BadgeTone }) {
  const { i18n } = useLingui();
  const label = transactionLabel(i18n, kind);
  return tone ? <Badge tone={tone}>{label}</Badge> : <Tag>{label}</Tag>;
}
