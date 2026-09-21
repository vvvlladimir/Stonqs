import { Trans, useLingui } from "@lingui/react/macro";
import { Empty, ErrorText, Panel } from "../../components/ui";
import type { ImportMapping, ImportPreviewData, ParseConfig } from "../../lib/types";
import { FilePreview } from "./FilePreview";

/** One question: which file. What it means is the next step's business. */
export function FileStep({
  loading,
  onPick,
  error,
  preview,
  config,
  mapping,
}: {
  loading: boolean;
  onPick: () => void;
  error: Error | null;
  preview: ImportPreviewData | null;
  config: ParseConfig | null;
  mapping: ImportMapping | null;
}) {
  const { t } = useLingui();
  if (!(config && mapping && preview)) {
    return (
      <Panel title={t`Broker export`}>
        <Empty
          title={t`No file chosen yet`}
          action={
            <button className="btn" onClick={onPick} disabled={loading}>
              {loading ? t`Reading…` : t`Choose a file`}
            </button>
          }
        >
          <Trans>
            A CSV or TXT from the broker's web office, or an Interactive Brokers Flex Query in XML — that one
            lays itself out and asks only for the account. The file is only read: nothing in the database
            changes before the last step.
          </Trans>
        </Empty>
        <ErrorText error={error} />
      </Panel>
    );
  }

  return (
    <>
      <FilePreview preview={preview} mapping={mapping} loading={loading} onPick={onPick} />
      <ErrorText error={error} />
    </>
  );
}
