import { Trans, useLingui } from "@lingui/react/macro";
import { Empty } from "../../components/ui";
import type { ReportsData } from "../../lib/types";
import { Disposals } from "./Disposals";
import { GainsByInstrument } from "./GainsByInstrument";
import { GainsByYear } from "./GainsByYear";

/** Realized result over the window: yearly rollup, per instrument, then every disposal. */
export function Gains({ data }: { data: ReportsData }) {
  const { t } = useLingui();

  if (data.disposals.length === 0) {
    return (
      <Empty title={t`No closed trades in this period`}>
        <Trans>
          A realized result appears on a sale: while positions stay open the whole gain is unrealized and
          lives on the "Positions" screen. Widen the period if the trades happened earlier.
        </Trans>
      </Empty>
    );
  }

  return (
    <>
      {data.gains_by_year.length > 1 && <GainsByYear data={data} />}
      <GainsByInstrument data={data} />
      <Disposals data={data} />
    </>
  );
}
