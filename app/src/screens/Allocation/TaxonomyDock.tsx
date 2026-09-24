import { useLingui } from "@lingui/react/macro";
import { createPortal } from "react-dom";
import { PlusIcon } from "@phosphor-icons/react";
import { useMenu, type MenuItem } from "../../components/ui";
import { useDockSlot } from "../../lib/dock";
import type { InstalledTaxonomySet, TaxonomyData } from "../../lib/types";
import { hasGaps } from "./model";

/** Taxonomy tabs rendered into the app dock; in edit mode a tab also opens its menu. */
export function TaxonomyDock({
  taxonomies,
  selected,
  editing,
  securityIds,
  sets,
  onSelect,
  onCreate,
  onEdit,
  onImport,
  onImportSet,
  onGroup,
  onExport,
  onDelete,
}: {
  taxonomies: TaxonomyData[];
  selected: string | null;
  editing: boolean;
  securityIds: string[];
  /** Ready trees the installed plugins bring; empty when none is installed. */
  sets: InstalledTaxonomySet[];
  onSelect: (id: string) => void;
  onCreate: () => void;
  onEdit: (taxonomy: TaxonomyData) => void;
  onImport: (taxonomy: TaxonomyData) => void;
  onImportSet: (key: string, taxonomy: TaxonomyData) => void;
  onGroup: (taxonomy: TaxonomyData) => void;
  onExport: (taxonomy: TaxonomyData) => void;
  onDelete: (taxonomy: TaxonomyData) => void;
}) {
  const { t } = useLingui();
  const menu = useMenu();
  const dockSlot = useDockSlot();

  if (!dockSlot) return null;

  return (
    <>
      {createPortal(
        <div className="taxdock">
          <div className="seg" role="group" aria-label={t`Classification`}>
            {taxonomies.map((tree) => {
              const items: MenuItem[] = [
                { label: t`Edit…`, onSelect: () => onEdit(tree) },
                { label: t`Import from CSV…`, onSelect: () => onImport(tree) },
                // A plugin's ready tree extends this one exactly as a CSV does, so it sits in
                // the same place rather than getting a control of its own.
                ...sets.map((set) => ({
                  label: t`Import ${set.name}…`,
                  onSelect: () => onImportSet(set.key, tree),
                })),
                { label: t`Group by attribute…`, onSelect: () => onGroup(tree) },
                { label: t`Export to CSV…`, onSelect: () => onExport(tree) },
                { label: t`Delete…`, danger: true, onSelect: () => onDelete(tree) },
              ];
              return (
                <button
                  key={tree.id}
                  type="button"
                  aria-pressed={tree.id === selected}
                  aria-haspopup={editing ? "menu" : undefined}
                  onClick={(e) => {
                    onSelect(tree.id);
                    if (editing) menu.openFrom(tree.id, items, e.currentTarget);
                  }}
                >
                  {tree.name}
                  {hasGaps(tree, securityIds) && (
                    <i className="tab-dot" aria-label={t`something is unclassified`} />
                  )}
                </button>
              );
            })}
            <button type="button" aria-label={t`New classification`} onClick={onCreate}>
              <PlusIcon className="d-flex" />
            </button>
          </div>
        </div>,
        dockSlot,
      )}
      {menu.node}
    </>
  );
}
