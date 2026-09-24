import type { ReactNode } from "react";
import { msg } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { useMutation } from "@tanstack/react-query";
import { PencilSimpleIcon, TrashIcon } from "@phosphor-icons/react";
import { api } from "../../lib/api";
import { formatDay, formatMoney } from "../../lib/format";
import { alertDirectionLabel, securityEventKindLabel } from "../../lib/kinds";
import { affects, useInvalidate } from "../../lib/queries";
import { Badge, Buttons, DayMark, ErrorText, List, ListRow, Money, Num, Percent } from "../ui";
import type { AlertCrossing, AlertProblem, AlertRow, CrossingRow, SecurityEventRow } from "../../lib/types";
import { SecurityLink } from "./SecurityCardProvider";

const PROBLEMS: Record<AlertProblem, ReturnType<typeof msg>> = {
  missing_price: msg`No quote yet`,
  missing_rate: msg`No exchange rate for the quote currency`,
};

/** An arrow, not a word: the direction reads at a glance in a line of dates. */
const ARROWS: Record<AlertCrossing["direction"], string> = { UP: "↑", DOWN: "↓", REACHED: "•" };

/**
 * Triggers with where the price stands and a trail of their latest crossings. `named` prints the
 * instrument for a list across instruments; `onEdit` adds edit and delete.
 */
export function AlertList({
  rows,
  named,
  onEdit,
}: {
  rows: AlertRow[];
  named?: boolean;
  onEdit?: (row: AlertRow) => void;
}) {
  const { t, i18n } = useLingui();
  const invalidate = useInvalidate();
  const remove = useMutation({
    mutationFn: api.alertDelete,
    onSuccess: () => invalidate(...affects.alerts),
  });

  return (
    <>
      <List>
        {rows.map((row) => {
          const { alert, status } = row;
          const trigger: ReactNode =
            alert.kind === "DATE_REACHED" ? (
              <Trans>On {formatDay(alert.date ?? "")}</Trans>
            ) : (
              <>
                {alertDirectionLabel(i18n, alert.direction)}{" "}
                <Money value={alert.price ?? "0"} currency={alert.currency ?? ""} />
              </>
            );
          const standing: ReactNode = row.problem ? (
            <span className="neg">{i18n._(PROBLEMS[row.problem])}</span>
          ) : status?.distance && status.price ? (
            <Trans>
              <Percent value={status.distance} signed digits={1} tone={false} /> from now,{" "}
              <Money value={status.price} currency={alert.currency ?? ""} />
            </Trans>
          ) : undefined;

          return (
            <ListRow
              key={alert.id}
              top
              title={named ? <SecurityLink id={alert.security_id}>{row.symbol}</SecurityLink> : trigger}
              sub={
                <>
                  {named && trigger}
                  {named && standing && " · "}
                  {standing}
                  {alert.note && <> · {alert.note}</>}
                </>
              }
              foot={row.crossings.length > 0 ? <CrossingTrail crossings={row.crossings} /> : undefined}
              end={
                onEdit && (
                  <Buttons>
                    <button
                      type="button"
                      className="iconbtn iconbtn--sm"
                      title={t`Edit`}
                      aria-label={t`Edit`}
                      onClick={() => onEdit(row)}
                    >
                      <PencilSimpleIcon />
                    </button>
                    <button
                      type="button"
                      className="iconbtn iconbtn--sm iconbtn--danger"
                      title={t`Delete`}
                      aria-label={t`Delete`}
                      disabled={remove.isPending}
                      onClick={() => remove.mutate(alert.id)}
                    >
                      <TrashIcon />
                    </button>
                  </Buttons>
                )
              }
            />
          );
        })}
      </List>
      <ErrorText error={remove.error} />
    </>
  );
}

/** One rule's latest crossings on a line: arrow, day, close. */
function CrossingTrail({ crossings }: { crossings: AlertCrossing[] }) {
  return (
    <span className="dim">
      {crossings.map((c, index) => (
        <span key={c.id}>
          {index > 0 && " · "}
          {ARROWS[c.direction]} {formatDay(c.date)}
          {c.price && (
            <>
              {" "}
              <Money value={c.price} currency={c.currency ?? ""} />
            </>
          )}
        </span>
      ))}
    </span>
  );
}

/** The crossing log across rules, newest first; a crossing not looked at yet carries the dot.
 * `compact` drops the year from the date: inside a tile that third line costs a whole row. */
export function CrossingLog({ rows, compact }: { rows: CrossingRow[]; compact?: boolean }) {
  const { t } = useLingui();
  return (
    <List>
      {rows.map((row) => {
        const level = formatMoney(row.level ?? "0", row.currency ?? "");
        return (
          <ListRow
            key={row.id}
            lead={<DayMark date={row.date} year={!compact} />}
            title={<SecurityLink id={row.security_id}>{row.symbol}</SecurityLink>}
            sub={
              row.direction === "UP" ? (
                <Trans>↑ crossed {level} upward</Trans>
              ) : row.direction === "DOWN" ? (
                <Trans>↓ crossed {level} downward</Trans>
              ) : (
                <Trans>the date has arrived</Trans>
              )
            }
            value={row.price ? <Money value={row.price} currency={row.currency ?? ""} /> : undefined}
            end={!row.seen ? <i className="tab-dot" aria-label={t`not looked at yet`} /> : undefined}
          />
        );
      })}
    </List>
  );
}

/**
 * Notes, dividends and splits, newest first. A reported split is offered as a split to record,
 * since a reported one moves no quantity. `onEditNote` adds editing and deleting.
 */
export function EventList({
  rows,
  named,
  compact,
  onEditNote,
}: {
  rows: SecurityEventRow[];
  named?: boolean;
  /** Tile reading: no year on the date, and the kind joins the line under the instrument. */
  compact?: boolean;
  onEditNote?: (event: SecurityEventRow) => void;
}) {
  const { t, i18n } = useLingui();
  const invalidate = useInvalidate();
  const record = useMutation({
    mutationFn: (event: SecurityEventRow) =>
      api.corporateActionSave({
        security_id: event.security_id,
        date: event.date,
        ratio_from: event.ratio_from ?? "1",
        ratio_to: event.ratio_to ?? "1",
      }),
    onSuccess: () => invalidate(...affects.securities),
  });
  const remove = useMutation({
    mutationFn: api.securityEventDelete,
    onSuccess: () => invalidate(...affects.alerts),
  });

  return (
    <>
      <List>
        {rows.map((event) => {
          const figure =
            event.kind === "DIVIDEND" ? (
              <Trans>
                <Money value={event.amount ?? "0"} currency={event.currency ?? ""} /> per share
              </Trans>
            ) : event.kind === "SPLIT" ? (
              <>
                <Num>{event.ratio_from ?? ""}</Num> → <Num>{event.ratio_to ?? ""}</Num>
              </>
            ) : null;
          const text = event.kind === "NOTE" ? event.note : null;

          return (
            <ListRow
              key={event.id}
              lead={<DayMark date={event.date} year={!compact} />}
              title={
                named ? (
                  <SecurityLink id={event.security_id}>{event.symbol}</SecurityLink>
                ) : (
                  securityEventKindLabel(i18n, event.kind)
                )
              }
              sub={
                <>
                  {named && <span>{securityEventKindLabel(i18n, event.kind)}</span>}
                  {figure}
                  {text}
                  {event.kind !== "NOTE" && event.note && <> · {event.note}</>}
                </>
              }
              end={
                <Buttons>
                  {event.kind === "SPLIT" &&
                    (event.recorded ? (
                      <Badge tone="in">
                        <Trans>Recorded</Trans>
                      </Badge>
                    ) : (
                      <button
                        type="button"
                        className="btn btn--sm"
                        disabled={record.isPending}
                        title={t`Adds it to the instrument's splits, which restates the lots`}
                        onClick={() => record.mutate(event)}
                      >
                        <Trans>Record split</Trans>
                      </button>
                    ))}
                  {onEditNote && (
                    <>
                      <button
                        type="button"
                        className="iconbtn iconbtn--sm"
                        title={t`Edit note`}
                        aria-label={t`Edit note`}
                        onClick={() => onEditNote(event)}
                      >
                        <PencilSimpleIcon />
                      </button>
                      <button
                        type="button"
                        className="iconbtn iconbtn--sm iconbtn--danger"
                        title={t`Delete`}
                        aria-label={t`Delete`}
                        disabled={remove.isPending}
                        onClick={() => remove.mutate(event.id)}
                      >
                        <TrashIcon />
                      </button>
                    </>
                  )}
                </Buttons>
              }
            />
          );
        })}
      </List>
      <ErrorText error={record.error ?? remove.error} />
    </>
  );
}
