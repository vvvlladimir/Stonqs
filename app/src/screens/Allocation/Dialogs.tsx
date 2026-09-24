import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { open as openFile, save as saveFile } from "@tauri-apps/plugin-dialog";
import { api } from "../../lib/api";
import type {
  AllocationTarget,
  NodeMember,
  TaxonomyData,
  TaxonomyNode,
  TaxonomyPreview,
} from "../../lib/types";
import { AssignDialog } from "../../components/domain/AssignDialog";
import { useSecurityCard } from "../../components/domain/SecurityCardProvider";
import { GroupDialog } from "./GroupDialog";
import { ImportDialog } from "./ImportDialog";
import { NodeDialog } from "./NodeDialog";
import { DeleteTaxonomyModal, TaxonomyDialog } from "./TaxonomyDialog";
import { UNCLASSIFIED } from "./model";

interface Options {
  taxonomy: TaxonomyData | null;
  target: AllocationTarget | null;
  /** Node the screen currently stands on: the parent of a new category. */
  levelKey: string | null;
  members: NodeMember[] | undefined;
  /** A tree, its weights and the plan built from them always move together. */
  onChanged: () => void;
  /** Select a taxonomy and return to its root. */
  onSelect: (id: string) => void;
  onDeleted: (id: string) => void;
  onNodeDeleted: (id: string) => void;
}

/**
 * Every modal of the screen with its own state, in the shape of `useMenu`:
 * the screen opens one by name and renders `node` once.
 */
export function useAllocationDialogs(opts: Options) {
  const card = useSecurityCard();
  const [assigning, setAssigning] = useState<string | null>(null);
  const [nodeDialog, setNodeDialog] = useState<{ node: TaxonomyNode | null } | null>(null);
  const [taxonomyDialog, setTaxonomyDialog] = useState<{ taxonomy: TaxonomyData | null } | null>(null);
  const [removing, setRemoving] = useState<TaxonomyData | null>(null);
  const [grouping, setGrouping] = useState<TaxonomyData | null>(null);
  // A file is a path and a plugin's set is bytes; the dialog above them is the same one, so the
  // difference is carried here rather than in two dialogs.
  const [importing, setImporting] = useState<{
    source: { path: string } | { content: number[] };
    preview: TaxonomyPreview;
    into: TaxonomyData | null;
  } | null>(null);
  const [busy, setBusy] = useState<string | null>(null);

  const saveTaxonomy = useMutation({
    mutationFn: api.taxonomySave,
    // Select the new taxonomy immediately; tabs are sorted by name.
    onSuccess: (saved) => {
      const id = (saved as { id?: string } | null)?.id;
      if (id) opts.onSelect(id);
      opts.onChanged();
    },
  });

  const deleteTaxonomy = useMutation({
    mutationFn: api.taxonomyDelete,
    onSuccess: (_data, id) => {
      opts.onDeleted(id);
      opts.onChanged();
    },
  });

  const pickImport = async (into: TaxonomyData | null) => {
    const path = await openFile({
      multiple: false,
      filters: [{ name: "CSV", extensions: ["csv"] }],
    });
    if (typeof path !== "string") return;
    setBusy("import");
    try {
      setImporting({ source: { path }, into, preview: await api.taxonomyImportPreviewPath(path) });
    } finally {
      setBusy(null);
    }
  };

  /** The same import, over a ready set a plugin brought: same preview, same commit, same dialog. */
  const importSet = async (key: string, into: TaxonomyData | null) => {
    const [plugin, set] = key.split("/");
    setBusy("import");
    try {
      const content = await api.pluginTaxonomyCsv(plugin, set);
      setImporting({ source: { content }, into, preview: await api.taxonomyImportPreview(content) });
    } finally {
      setBusy(null);
    }
  };

  const exportCsv = async (t: TaxonomyData) => {
    const path = await saveFile({
      defaultPath: `${t.name}.csv`,
      filters: [{ name: "CSV", extensions: ["csv"] }],
    });
    if (!path) return;
    setBusy("export");
    try {
      await api.taxonomyExportSave(t.id, path);
    } finally {
      setBusy(null);
    }
  };

  const taxonomy = opts.taxonomy;

  const node = (
    <>
      {assigning && taxonomy && (
        <AssignDialog
          taxonomy={taxonomy}
          subjectId={assigning}
          name={subjectName(opts.members, assigning)}
          onClose={() => setAssigning(null)}
          onSaved={() => {
            setAssigning(null);
            opts.onChanged();
          }}
        />
      )}

      {nodeDialog && taxonomy && (
        <NodeDialog
          taxonomy={taxonomy}
          node={nodeDialog.node}
          parentId={
            nodeDialog.node
              ? nodeDialog.node.parent_id
              : opts.levelKey === UNCLASSIFIED
                ? null
                : (opts.levelKey ?? null)
          }
          target={opts.target}
          onClose={() => setNodeDialog(null)}
          onSaved={() => {
            setNodeDialog(null);
            opts.onChanged();
          }}
          onDeleted={(id) => {
            setNodeDialog(null);
            opts.onNodeDeleted(id);
            opts.onChanged();
          }}
        />
      )}

      {importing && (
        <ImportDialog
          preview={importing.preview}
          into={importing.into}
          busy={busy === "commit"}
          onClose={() => setImporting(null)}
          onImport={async (name, withTargets) => {
            setBusy("commit");
            try {
              const into = importing.into?.id ?? null;
              const saved =
                "path" in importing.source
                  ? await api.taxonomyImportCommitPath(importing.source.path, name, into, withTargets)
                  : await api.taxonomyImportCommit(importing.source.content, name, into, withTargets);
              setImporting(null);
              opts.onSelect(saved.id);
              opts.onChanged();
            } finally {
              setBusy(null);
            }
          }}
        />
      )}

      {grouping && (
        <GroupDialog
          into={grouping}
          busy={busy === "group"}
          onClose={() => setGrouping(null)}
          onGroup={async (attributeId) => {
            setBusy("group");
            try {
              await api.taxonomyGroupCommit(attributeId, grouping.id, null);
              setGrouping(null);
              opts.onSelect(grouping.id);
              opts.onChanged();
            } finally {
              setBusy(null);
            }
          }}
        />
      )}

      {taxonomyDialog && (
        <TaxonomyDialog
          taxonomy={taxonomyDialog.taxonomy}
          busy={busy === "create"}
          onClose={() => setTaxonomyDialog(null)}
          onSave={async (name, attributeId) => {
            // Naming an attribute creates the tree and fills it in one command: a fresh tree
            // classifies nothing, so there is nothing a grouping could overwrite.
            if (attributeId) {
              setBusy("create");
              try {
                const saved = await api.taxonomyGroupCommit(attributeId, null, name);
                setTaxonomyDialog(null);
                opts.onSelect(saved.id);
                opts.onChanged();
              } finally {
                setBusy(null);
              }
              return;
            }
            saveTaxonomy.mutate({ id: taxonomyDialog.taxonomy?.id ?? null, name });
            setTaxonomyDialog(null);
          }}
        />
      )}

      {removing && (
        <DeleteTaxonomyModal
          taxonomy={removing}
          busy={deleteTaxonomy.isPending}
          onClose={() => setRemoving(null)}
          onConfirm={() => {
            deleteTaxonomy.mutate(removing.id);
            setRemoving(null);
          }}
        />
      )}
    </>
  );

  return {
    node,
    busy,
    openCard: card.open,
    openAssign: setAssigning,
    openNode: (node: TaxonomyNode | null) => setNodeDialog({ node }),
    openTaxonomy: (taxonomy: TaxonomyData | null) => setTaxonomyDialog({ taxonomy }),
    openDelete: setRemoving,
    openGroup: setGrouping,
    pickImport,
    importSet,
    exportCsv,
  };
}

function subjectName(members: NodeMember[] | undefined, subjectId: string): string {
  const member = members?.find((m) => m.subject_id === subjectId);
  if (!member) return subjectId;
  return member.name ? `${member.symbol} · ${member.name}` : member.symbol;
}
