import { plural } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { Banner, Buttons, Panel } from "../../components/ui";
import { usePlugins } from "../../lib/queries";
import type { ImportMapping, ImportPreviewData } from "../../lib/types";
import { FileTable } from "./FileTable";

/** How many rows read like a sample of the file; the rest are one click away. */
const SHOWN = 20;

/**
 * The file as it is written, before anything is interpreted: it answers "did we even
 * read this right" — the delimiter, the header row, and which column is which.
 */
export function FilePreview({
  preview,
  mapping,
  loading,
  onPick,
}: {
  preview: ImportPreviewData;
  mapping: ImportMapping;
  loading: boolean;
  onPick: () => void;
}) {
  const { t } = useLingui();
  const [all, setAll] = useState(false);
  const plugins = usePlugins();

  const rows = all ? preview.rows : preview.rows.slice(0, SHOWN);
  const more = preview.rows.length > SHOWN;
  // The table below is what the reader produced, not what was on disk — which is worth saying,
  // because the columns in it are the app's own and not the ones the file had.
  const readBy = preview.reader
    ? (plugins.data?.plugins.find((p) => p.id === preview.reader?.split("/")[0])?.name ??
      preview.reader.split("/")[0])
    : null;

  return (
    <Panel
      title={t`What is in the file`}
      note={`${plural(preview.rows.length, { one: "# row", other: "# rows" })} · ${plural(
        preview.headers.length,
        { one: "# column", other: "# columns" },
      )}`}
      tools={
        <Buttons>
          {more && (
            <button className="iconbtn iconbtn--sm" onClick={() => setAll(!all)}>
              {all ? t`collapse to ${SHOWN}` : t`show all ${preview.rows.length}`}
            </button>
          )}
          <button className="btn btn--ghost" onClick={onPick} disabled={loading}>
            {loading ? t`Reading…` : t`Choose another`}
          </button>
        </Buttons>
      }
      table
    >
      {readBy && (
        <Banner>
          <Trans>Read by the {readBy} plugin, which turned it into the app's own transaction file.</Trans>
        </Banner>
      )}
      {preview.reader_warnings?.map((warning, index) => (
        // The reader's own words, in its own language: the app has no table to translate a
        // stranger's code from, so it is shown beside the sentence rather than instead of it.
        <Banner key={index}>
          {warning.message} ({warning.code})
        </Banner>
      ))}
      <FileTable preview={preview} mapping={mapping} rows={rows} />
    </Panel>
  );
}
