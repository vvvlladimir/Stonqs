import { plural } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { useMemo } from "react";
import { Badge, Banner, CheckField, Field, Form, List, ListRow, Panel } from "../../components/ui";
import { useQuoteProviders } from "../../lib/queries";
import type { ImportOptions, ImportPreviewData, ImportResult, RowOverride } from "../../lib/types";
import { ProblemList } from "./ProblemList";
import { RowTable } from "./RowTable";

/** Choose a quote source for instruments not resolved during import. */
function NewSecuritySource({
  options,
  onOptions,
}: {
  options: ImportOptions;
  onOptions: (o: ImportOptions) => void;
}) {
  const { t } = useLingui();
  const providers = useQuoteProviders();

  return (
    <Field
      label={t`Quote source for unidentified instruments`}
      hint={t`Instruments identified on the "Instruments" step arrive with their own source and ticker. This covers only those left as they are in the file: their symbol goes to the provider unchanged. "Manual" means prices are typed in, and the automatic refresh skips such instruments`}
      placeholder={t`— manual —`}
      options={(providers.data ?? []).map((id) => ({ value: id, label: id }))}
      value={
        options.new_security_source === undefined
          ? (providers.data?.[0] ?? "")
          : (options.new_security_source ?? "")
      }
      onChange={(source) => onOptions({ ...options, new_security_source: source || null })}
    />
  );
}

/**
 * The last step is the decision and its evidence on one screen: what will be written sits
 * above, broken down by why a row is or is not part of it, and every row it came from below.
 */
export function CommitStep({
  preview,
  overrides,
  onOverrides,
  options,
  onOptions,
  onCommit,
  pending,
  error,
  result,
}: {
  preview: ImportPreviewData;
  overrides: RowOverride[];
  onOverrides: (next: RowOverride[]) => void;
  options: ImportOptions;
  onOptions: (o: ImportOptions) => void;
  onCommit: () => void;
  pending: boolean;
  error: Error | null;
  result: ImportResult | null;
}) {
  const { t } = useLingui();
  const s = preview.summary;
  const newSecurities = preview.symbols.filter((x) => x.required && !x.security_id);
  const willWrite =
    s.ready +
    (options.create_missing_securities ? s.unknown_securities : 0) +
    (options.import_duplicates ? s.duplicates : 0);

  const warnings = useMemo(
    () => preview.rows.flatMap((r) => r.problems).filter((p) => p.severity === "WARNING"),
    [preview.rows],
  );

  return (
    <>
      {result && (
        <Banner tone="info">
          <Trans>
            {result.imported} written, {result.skipped} skipped.
          </Trans>
          {result.created_securities.length > 0 &&
            t` Instruments created: ${result.created_securities.join(", ")}.`}{" "}
          <Trans>The file is in the database — writing it again adds nothing.</Trans>
        </Banner>
      )}

      <Panel title={t`What will be written`} note={t`${willWrite} of ${s.total}`}>
        {/* One line per reason a row is in or out, with its switch where the reason is stated. */}
        <List>
          <ListRow
            title={t`Ready to write`}
            sub={t`parsed and matching nothing in the database`}
            value={<Badge tone="in">{s.ready}</Badge>}
          />
          {s.unknown_securities > 0 && (
            <ListRow
              title={t`The instrument is not in the database`}
              sub={
                newSecurities.length > 0
                  ? t`will be created: ${newSecurities.map((x) => x.planned?.symbol ?? x.value).join(", ")}`
                  : t`the file's instrument does not exist yet`
              }
              value={<Badge tone="warn">{s.unknown_securities}</Badge>}
              end={
                <CheckField
                  label={t`create`}
                  checked={options.create_missing_securities}
                  onChange={(on) => onOptions({ ...options, create_missing_securities: on })}
                />
              }
            />
          )}
          {s.duplicates > 0 && (
            <ListRow
              title={t`Duplicates`}
              sub={t`the transaction's fingerprint is already in the database`}
              value={<Badge>{s.duplicates}</Badge>}
              end={
                <CheckField
                  label={t`write`}
                  checked={options.import_duplicates}
                  onChange={(on) => onOptions({ ...options, import_duplicates: on })}
                />
              }
            />
          )}
          {s.ignored > 0 && (
            <ListRow
              title={t`Skipped by decision`}
              sub={t`these transaction kinds are marked "do not import" on the "Parsing" step`}
              value={<Badge>{s.ignored}</Badge>}
            />
          )}
          {s.invalid > 0 && (
            <ListRow
              title={t`Not parsed`}
              sub={t`will be skipped: fix it on the "Parsing" step, or row by row below`}
              value={<Badge tone="out">{s.invalid}</Badge>}
            />
          )}
          {s.warnings > 0 && (
            <ListRow
              title={t`With notices`}
              sub={t`they are written, but worth a look`}
              value={<Badge tone="warn">{s.warnings}</Badge>}
            />
          )}
        </List>

        <Form
          onSubmit={onCommit}
          busy={pending}
          error={error}
          ready={willWrite > 0 && result === null}
          submitLabel={t`Write ${plural(willWrite, { one: "# row", other: "# rows" })}`}
          busyLabel={t`Writing…`}
        >
          {options.create_missing_securities && s.unknown_securities > 0 && (
            <NewSecuritySource options={options} onOptions={onOptions} />
          )}
          <p className="muted">
            <Trans>
              The write is one transaction: a half-imported file can be neither finished (half the rows would
              become duplicates) nor rolled back. Importing the same file again does nothing — duplicates are
              cut off by the transaction fingerprint.
            </Trans>
          </p>
        </Form>
      </Panel>

      <ProblemList title={t`Row notices`} problems={warnings} />

      <RowTable preview={preview} overrides={overrides} onOverrides={onOverrides} />
    </>
  );
}
