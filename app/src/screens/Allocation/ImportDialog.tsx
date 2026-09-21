import { plural } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import {
  CheckField,
  DataTable,
  ErrorText,
  Field,
  FormDialog,
  Metric,
  Metrics,
  Scrolly,
} from "../../components/ui";
import type { TaxonomyData, TaxonomyPreview } from "../../lib/types";
import { countBy, depthOf } from "./model";

/** Preview taxonomy imports before writing them to storage. */
export function ImportDialog({
  preview,
  into,
  busy,
  onClose,
  onImport,
}: {
  preview: TaxonomyPreview;
  into: TaxonomyData | null;
  busy: boolean;
  onClose: () => void;
  onImport: (name: string, withTargets: boolean) => void;
}) {
  const { t } = useLingui();
  const [name, setName] = useState(preview.name);
  const [withTargets, setWithTargets] = useState(true);
  const matched = preview.assignments.filter((a) => a.security_id !== null);
  const missing = preview.assignments.filter((a) => a.security_id === null);
  const targets = preview.nodes.filter((n) => n.target !== null);
  const errors = preview.problems.filter((p) => p.severity === "ERROR");

  return (
    <FormDialog
      title={into ? t`Import into "${into.name}"` : t`Import a classification`}
      onClose={onClose}
      onSubmit={() => onImport(name, withTargets)}
      busy={busy}
      busyLabel={t`Importing…`}
      submitLabel={into ? t`Add` : t`Import`}
      ready={(into !== null || name.trim() !== "") && preview.nodes.length > 0}
      wide
    >
      {into ? (
        <p className="panel__note">
          <Trans>
            The categories from the file are added to "{into.name}". One that already exists under the same
            name and in the same place of the tree is reused rather than duplicated; an instrument's split
            across such a category is taken from the file.
          </Trans>
        </p>
      ) : (
        <Field
          label={t`Name`}
          hint={t`Taken from the file. The tree is created fresh; nothing is overwritten.`}
        >
          <input value={name} autoFocus onChange={(e) => setName(e.target.value)} />
        </Field>
      )}

      <Metrics>
        <Metric
          label={t`Categories`}
          value={String(preview.nodes.length)}
          hint={t`levels: ${depthOf(preview)}`}
        />
        <Metric
          label={t`Instruments found`}
          value={String(matched.length)}
          hint={t`of ${preview.assignments.length} in the file`}
        />
        <Metric
          label={t`Not found`}
          value={String(missing.length)}
          tone={missing.length > 0 ? "negative" : "neutral"}
          hint={missing.length > 0 ? t`the split will not be written` : t`all matched`}
        />
        <Metric
          label={t`Target weights`}
          value={String(targets.length)}
          hint={targets.length > 0 ? t`importing them is optional` : t`none in the file`}
        />
      </Metrics>

      {targets.length > 0 && (
        <CheckField
          checked={withTargets}
          onChange={setWithTargets}
          label={
            <>
              <Trans>Create a target from the file's weights</Trans>
              <span className="dim">
                {" "}
                <Trans>
                  — weights in such files are given inside the parent ("equities 42 %, of which the core is 88
                  %"), and that is how they are stored
                </Trans>
              </span>
            </>
          }
        />
      )}

      {missing.length > 0 && (
        <div>
          <p className="panel__note">
            <Trans>
              These rows create a category but no split — the instrument is in the database under neither ISIN
              nor ticker:
            </Trans>
          </p>
          <Scrolly max={160}>
            <DataTable
              card={false}
              rows={missing.slice(0, 40)}
              rowKey={(a) => `${a.row}-${a.label}`}
              columns={[
                { key: "label", align: "left", className: "nm", cell: (a) => a.label },
                {
                  key: "id",
                  align: "left",
                  className: "sub",
                  cell: (a) => [a.symbol, a.isin].filter(Boolean).join(" · "),
                },
                { key: "path", cell: (a) => a.path.join(" › ") },
              ]}
            />
          </Scrolly>
          {missing.length > 40 && (
            <p className="panel__note">
              <Trans>…and {missing.length - 40} more.</Trans>
            </p>
          )}
        </div>
      )}

      {matched.length > 0 && (
        <p className="panel__note">
          <Trans>
            Matches: {countBy(matched, "isin")} by ISIN, {countBy(matched, "symbol")} by ticker,{" "}
            {countBy(matched, "name")} by name.
          </Trans>
        </p>
      )}

      {errors.length > 0 && (
        <ErrorText>
          <Trans>
            {plural(errors.length, { one: "# row", other: "# rows" })} not parsed: {errors[0].message}
          </Trans>
        </ErrorText>
      )}

      <p className="panel__note">
        <Trans>
          Columns were detected automatically: levels — {preview.config.levels.join(", ") || t`not found`};
          share — {preview.config.weight ?? t`none`}; target — {preview.config.target ?? t`none`}; ticker —{" "}
          {preview.config.symbol ?? t`none`}; ISIN — {preview.config.isin ?? t`none`}.
        </Trans>
      </p>
    </FormDialog>
  );
}
