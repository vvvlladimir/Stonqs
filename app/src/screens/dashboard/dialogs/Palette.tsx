import { useLingui } from "@lingui/react/macro";
import { List, ListRow, Modal } from "../../../components/ui";
import { WIDGET_READ_LABELS } from "../../../lib/kinds";
import { useWidgetCatalog, type WidgetDef } from "../widgets";

/** Widget catalog grouped for visual browsing. */
export function Palette({ onClose, onPick }: { onClose: () => void; onPick: (type: string) => void }) {
  const { t, i18n } = useLingui();
  // Group by the translated name: the catalog's sections are what the user reads.
  const catalog = useWidgetCatalog();
  const groups = new Map<string, [string, WidgetDef][]>();
  for (const [type, def] of Object.entries(catalog.all)) {
    const group = i18n._(def.group);
    groups.set(group, [...(groups.get(group) ?? []), [type, def]]);
  }

  // Placing a plugin's tile is the consent to what it reads, so the palette says so before the
  // first time it does (ADR-0083).
  const describe = (def: WidgetDef) => {
    const description = i18n._(def.description);
    if (!def.plugin) return description;
    const plugin = def.plugin.name;
    const reads = def.plugin.reads.map((read) => i18n._(WIDGET_READ_LABELS[read])).join(", ");
    return reads
      ? t`${description} From ${plugin}, which is given ${reads}.`
      : t`${description} From ${plugin}, which is given no portfolio data.`;
  };

  return (
    <Modal title={t`Add widget`} onClose={onClose} wide>
      {[...groups].map(([group, list]) => (
        <div key={group}>
          <div className="group-label">{group}</div>
          <List variant="cards">
            {list.map(([type, def]) => {
              const Icon = def.icon;
              return (
                <ListRow
                  key={type}
                  box
                  top
                  wrap
                  lead={<Icon />}
                  title={i18n._(def.label)}
                  sub={describe(def)}
                  onClick={() => onPick(type)}
                />
              );
            })}
          </List>
        </div>
      ))}
    </Modal>
  );
}
