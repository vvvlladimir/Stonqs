import { plural } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { Buttons, Panel } from "../../components/ui";
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

  const rows = all ? preview.rows : preview.rows.slice(0, SHOWN);
  const more = preview.rows.length > SHOWN;

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
      <FileTable preview={preview} mapping={mapping} rows={rows} />
    </Panel>
  );
}
