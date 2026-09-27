import { Trans, useLingui } from "@lingui/react/macro";
import { Banner } from "../../components/ui";
import type { SecurityRow } from "../../lib/types";

/** What is wrong across the directory, each with its remedy. */
export function Banners({
  broken,
  mismatched,
  fileError,
  identifying,
  onIdentifyAll,
}: {
  broken: SecurityRow[];
  mismatched: SecurityRow[];
  fileError: string | null;
  identifying: boolean;
  onIdentifyAll: () => void;
}) {
  const { t } = useLingui();
  return (
    <>
      {fileError !== null && (
        <Banner tone="bad">
          <Trans>Could not read or write the file: {fileError}</Trans>
        </Banner>
      )}
      {broken.length > 0 && (
        <Banner
          action={
            <button className="btn btn--sm" disabled={identifying} onClick={onIdentifyAll}>
              {identifying ? t`Identifying…` : t`Identify all (${broken.length})`}
            </button>
          }
        >
          <Trans>
            <b>{broken.length}</b> instruments will get no quotes: their ticker field holds an ISIN — the code
            of the instrument, not of a listing. The provider will always answer 404.
          </Trans>
        </Banner>
      )}
      {mismatched.length > 0 && (
        <Banner tone="info">
          <Trans>
            <b>{mismatched.length}</b> instruments are quoted in a currency other than their own. The maths is
            still right — valuation converts the price at the day's rate — but it usually means the wrong
            venue is selected: "Venues" in the row shows the other exchanges.
          </Trans>
        </Banner>
      )}
    </>
  );
}
