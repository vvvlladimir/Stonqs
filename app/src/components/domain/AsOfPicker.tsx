import { useState } from "react";
import { CalendarBlankIcon, CaretUpDownIcon, ClockCounterClockwiseIcon } from "@phosphor-icons/react";
import { Trans, useLingui } from "@lingui/react/macro";

import { useAsOf } from "../../lib/asOf";
import { endOfPreviousMonth, endOfPreviousYear } from "../../lib/dates";
import { today } from "../../lib/api";
import { formatDay, formatDayNumeric } from "../../lib/format";
import { Banner, Field, ListRow, Modal } from "../ui";
import type { DateString } from "../../lib/types";

/**
 * The date the reading screens answer for, beside the scope picker and wearing its look: two
 * lenses, one control shape. The date is not stored — see `lib/asOf`.
 */
export function AsOfPicker() {
  const { t } = useLingui();
  const { date, isToday, set, reset } = useAsOf();
  const [open, setOpen] = useState(false);
  const now = today();

  const choose = (value: DateString) => {
    set(value);
    setOpen(false);
  };

  const Icon = isToday ? CalendarBlankIcon : ClockCounterClockwiseIcon;

  return (
    <>
      <button
        type="button"
        className="scope scope--row"
        data-tip={isToday ? t`Today` : t`Past date`}
        onClick={() => setOpen(true)}
      >
        <Icon className="scope__icon" />
        <span className="scope__label">{formatDayNumeric(date)}</span>
        <CaretUpDownIcon className="scope__caret" />
      </button>

      {open && (
        <Modal title={t`Date`} onClose={() => setOpen(false)}>
          <div className="smenu">
            <ListRow
              lead={<CalendarBlankIcon />}
              title={t`Today`}
              sub={formatDay(now)}
              on={isToday}
              onClick={() => {
                reset();
                setOpen(false);
              }}
            />
            <ListRow
              lead={<ClockCounterClockwiseIcon />}
              title={t`End of last month`}
              sub={formatDay(endOfPreviousMonth(now))}
              on={date === endOfPreviousMonth(now)}
              onClick={() => choose(endOfPreviousMonth(now))}
            />
            <ListRow
              lead={<ClockCounterClockwiseIcon />}
              title={t`End of last year`}
              sub={formatDay(endOfPreviousYear(now))}
              on={date === endOfPreviousYear(now)}
              onClick={() => choose(endOfPreviousYear(now))}
            />
          </div>
          <Field label={t`Another date`} hint={t`Values, holdings and periods are read as of this day.`}>
            <input
              type="date"
              value={date}
              max={now}
              onChange={(e) => e.target.value && set(e.target.value)}
            />
          </Field>
        </Modal>
      )}
    </>
  );
}

/**
 * Says that the screen is not showing the present. Without it a portfolio read at a past date
 * looks like one whose prices stopped updating.
 */
export function AsOfBanner() {
  const { isToday, date, reset } = useAsOf();
  if (isToday) return null;
  return (
    <div className="timelens">
      <Banner
        tone="info"
        action={
          <button type="button" className="btn btn--ghost btn--sm" onClick={reset}>
            <Trans>Back to today</Trans>
          </button>
        }
      >
        <Trans>Showing the portfolio as of {formatDay(date)}.</Trans>
      </Banner>
    </div>
  );
}
