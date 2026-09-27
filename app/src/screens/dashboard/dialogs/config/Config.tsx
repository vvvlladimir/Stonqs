import { useState } from "react";
import { useLingui } from "@lingui/react/macro";
import { Field, FormDialog } from "../../../../components/ui";
import type { Widget } from "../../../../lib/uiState";
import { GRID_COLS, limitsOf } from "../../grid";
import { useWidgetCatalog, type Field as WidgetField, type WidgetDef } from "../../widgets";
import { trackOf, type Track } from "../../widgets/model";
import { BriefFields } from "./BriefFields";
import type { FieldsCtx } from "./context";
import { PeriodFields } from "./PeriodFields";
import { ShownFields } from "./ShownFields";
import { SubjectFields } from "./SubjectFields";
import { TrackFields } from "./TrackFields";

interface ConfigProps {
  widget: Widget;
  onClose: () => void;
  onSave: (widget: Widget) => void;
}

/** Shared configuration dialog for all widget types. */
export function Config(props: ConfigProps) {
  const def = useWidgetCatalog().of(props.widget.type);
  return def ? <WidgetConfig {...props} def={def} /> : null;
}

function WidgetConfig({ widget, onClose, onSave, def }: ConfigProps & { def: WidgetDef }) {
  const { t, i18n } = useLingui();
  const [draft, setDraft] = useState<Widget>({ ...widget, cfg: { ...widget.cfg } });

  const has = (field: WidgetField) => def.fields.includes(field);
  // A tile that picks its subject offers only that subject's settings: a goal tile has no
  // withdrawal rate, and a dialog listing all three at once asks about two of them for nothing.
  const tracks = (track: Track) => has(track) && (!has("track") || trackOf(draft.cfg) === track);
  const set = (key: string, value: unknown) => setDraft({ ...draft, cfg: { ...draft.cfg, [key]: value } });
  const text = (key: string) => (typeof draft.cfg[key] === "string" ? (draft.cfg[key] as string) : "");
  const min = limitsOf(draft, def);
  const ctx: FieldsCtx = { def, draft, setDraft, has, tracks, set, text };

  return (
    <FormDialog
      title={i18n._(def.label)}
      onClose={onClose}
      // A tile narrower than its own minimum is a minimum nothing enforces; the width is raised
      // to it here rather than waiting for the next drag to notice.
      onSubmit={() => onSave({ ...draft, w: Math.max(draft.w, min.w) })}
    >
      <ShownFields ctx={ctx} />
      <TrackFields ctx={ctx} />
      <PeriodFields ctx={ctx} />
      <SubjectFields ctx={ctx} />
      <BriefFields ctx={ctx} />

      {has("count") && (
        <Field label={t`Rows`}>
          <input
            type="number"
            min={3}
            max={20}
            value={Number(draft.cfg.count) || 6}
            onChange={(e) => set("count", Math.min(Math.max(Number(e.target.value) || 6, 3), 20))}
          />
        </Field>
      )}

      {/* Offered by every tile rather than named in `fields`: how small a widget may get is a
          property of the board, not of what the widget shows. */}
      {!def.plain && (
        <Field
          label={t`Smallest width, in twelfths of the board`}
          hint={t`Where a drag stops, and how the tile behaves on a phone: four twelfths or more takes the whole row there instead of half of it. Left at ${def.min.w}, the widget's own.`}
        >
          <input
            type="number"
            min={1}
            max={GRID_COLS}
            value={min.w}
            onChange={(e) => {
              const twelfths = Math.round(Number(e.target.value));
              set("min_w", twelfths >= 1 && twelfths <= GRID_COLS ? twelfths : null);
            }}
          />
        </Field>
      )}
    </FormDialog>
  );
}
