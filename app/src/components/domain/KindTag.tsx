import { useLingui } from "@lingui/react/macro";
import { Badge, Tag, type BadgeTone } from "../ui";
import { transactionLabel } from "../../lib/kinds";
import type { TransactionKind } from "../../lib/types";

/** A transaction kind's name; `tone` makes it a badge coloured by cash direction. */
export function KindTag({ kind, tone }: { kind: TransactionKind; tone?: BadgeTone }) {
  const { i18n } = useLingui();
  const label = transactionLabel(i18n, kind);
  return tone ? <Badge tone={tone}>{label}</Badge> : <Tag>{label}</Tag>;
}
