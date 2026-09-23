import { plural } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { DataTable, ErrorText, FormDialog, Metric, Metrics, Scrolly, Tag } from "../../components/ui";
import type { AttributePreview } from "../../lib/types";

const SHOWN = 40;

/** What an attribute file would fill in, shown before anything is written. */
export function AttributeImportDialog({
  preview,
  busy,
  onClose,
  onImport,
}: {
  preview: AttributePreview;
  busy: boolean;
  onClose: () => void;
  onImport: () => void;
}) {
  const { t } = useLingui();
  const matched = preview.rows.filter((r) => r.security_id !== null);
  const missing = preview.rows.filter((r) => r.security_id === null);
  const created = preview.attributes.filter((a) => a.attribute_id === null && a.values > 0);
  const errors = preview.problems.filter((p) => p.severity === "ERROR");
  const values = matched.reduce((total, row) => total + Object.keys(row.values).length, 0);

  const KIND_LABEL: Record<string, string> = {
    TEXT: t`text`,
    NUMBER: t`number`,
    DATE: t`date`,
  };

  return (
    <FormDialog
      title={t`Import attributes`}
      onClose={onClose}
      onSubmit={onImport}
      busy={busy}
      busyLabel={t`Importing…`}
      submitLabel={t`Import`}
      ready={values > 0}
      wide
    >
      <Metrics>
        <Metric
          label={t`Instruments found`}
          value={String(matched.length)}
          hint={t`of ${preview.rows.length} in the file`}
        />
        <Metric
          label={t`Not found`}
          value={String(missing.length)}
          tone={missing.length > 0 ? "negative" : "neutral"}
          hint={missing.length > 0 ? t`these rows are not written` : t`all matched`}
        />
        <Metric label={t`Values`} value={String(values)} hint={t`cells that will be filled in`} />
        <Metric
          label={t`New attributes`}
          value={String(created.length)}
          hint={created.length > 0 ? t`created on import` : t`all columns already exist`}
        />
      </Metrics>

      <div>
        <p className="panel__note">
          <Trans>
            Columns of the file. An attribute that already exists keeps the kind it was created with; a new
            one is read from its own values.
          </Trans>
        </p>
        <Scrolly max={160}>
          <DataTable
            card={false}
            rows={preview.attributes}
            rowKey={(a) => a.name}
            columns={[
              { key: "name", align: "left", className: "nm", cell: (a) => a.name },
              { key: "kind", align: "left", className: "sub", cell: (a) => KIND_LABEL[a.kind] ?? a.kind },
              {
                key: "state",
                cell: (a) =>
                  a.attribute_id === null ? (
                    <Tag>
                      <Trans>new</Trans>
                    </Tag>
                  ) : (
                    <span className="dim">
                      <Trans>exists</Trans>
                    </span>
                  ),
              },
              {
                key: "values",
                cell: (a) => plural(a.values, { one: "# value", other: "# values" }),
              },
            ]}
          />
        </Scrolly>
      </div>

      {missing.length > 0 && (
        <div>
          <p className="panel__note">
            <Trans>
              These rows are skipped — the instrument is in the database under neither ISIN nor ticker:
            </Trans>
          </p>
          <Scrolly max={160}>
            <DataTable
              card={false}
              rows={missing.slice(0, SHOWN)}
              rowKey={(r) => String(r.row)}
              columns={[
                { key: "label", align: "left", className: "nm", cell: (r) => r.label },
                {
                  key: "id",
                  align: "left",
                  className: "sub",
                  cell: (r) => [r.symbol, r.isin].filter(Boolean).join(" · "),
                },
              ]}
            />
          </Scrolly>
          {missing.length > SHOWN && (
            <p className="panel__note">
              <Trans>…and {missing.length - SHOWN} more.</Trans>
            </p>
          )}
        </div>
      )}

      {errors.length > 0 && (
        <ErrorText>
          <Trans>
            {plural(errors.length, { one: "# cell", other: "# cells" })} do not fit their attribute and are
            skipped: {errors[0].message}
          </Trans>
        </ErrorText>
      )}

      <p className="panel__note">
        <Trans>
          A column the file does not name is left as it is — importing one column never clears the others.
        </Trans>
      </p>
    </FormDialog>
  );
}
