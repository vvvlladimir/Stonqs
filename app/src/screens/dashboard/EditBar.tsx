import { Trans, useLingui } from "@lingui/react/macro";
import {
  CopyIcon,
  DownloadSimpleIcon,
  PencilSimpleIcon,
  TrashIcon,
  UploadSimpleIcon,
} from "@phosphor-icons/react";
import { InfoTip } from "../../components/ui";
import type { Dashboard as Board } from "../../lib/uiState";

/** Board-level actions, shown only while the layout is being edited. */
export function EditBar({
  board,
  onRename,
  onDuplicate,
  onExport,
  onImport,
  onDelete,
}: {
  board: Board;
  onRename: () => void;
  onDuplicate: () => void;
  /** Writes the board as a file; the same file `onImport` reads back. */
  onExport: () => void;
  onImport: () => void;
  onDelete: () => void;
}) {
  const { t } = useLingui();
  return (
    <div className="panel panel--bar">
      <div className="panel__head">
        <span className="panel__note inline">
          <span>
            <Trans>
              Editing <b>{board.name}</b>
            </Trans>
          </span>
          <InfoTip text={t`Drag a tile's header to move it, or any of its edges or corners to resize it.`} />
        </span>
        <div className="panel__tools">
          <button type="button" className="btn btn--ghost btn--sm" onClick={onRename}>
            <PencilSimpleIcon /> <Trans>Rename</Trans>
          </button>
          <button type="button" className="btn btn--ghost btn--sm" onClick={onDuplicate}>
            <CopyIcon /> <Trans>Duplicate</Trans>
          </button>
          {/* The arrow says where the board is going: out of the app on export, into it on
              import — not which way a download travels. */}
          <button type="button" className="btn btn--ghost btn--sm" onClick={onExport}>
            <UploadSimpleIcon /> <Trans>Export</Trans>
          </button>
          <button type="button" className="btn btn--ghost btn--sm" onClick={onImport}>
            <DownloadSimpleIcon /> <Trans>Import</Trans>
          </button>
          <button type="button" className="btn btn--danger btn--sm" onClick={onDelete}>
            <TrashIcon /> <Trans>Delete</Trans>
          </button>
        </div>
      </div>
    </div>
  );
}
