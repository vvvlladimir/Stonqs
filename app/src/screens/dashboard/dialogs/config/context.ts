import type { Widget } from "../../../../lib/uiState";
import type { Field as WidgetField, WidgetDef } from "../../widgets";
import type { Track } from "../../widgets/model";

/** What every section of the widget dialog reads and writes: the draft and its accessors. */
export interface FieldsCtx {
  def: WidgetDef;
  draft: Widget;
  setDraft: (widget: Widget) => void;
  has: (field: WidgetField) => boolean;
  /** A tile that picks its subject offers only that subject's settings. */
  tracks: (track: Track) => boolean;
  set: (key: string, value: unknown) => void;
  text: (key: string) => string;
}
