import { Trans, useLingui } from "@lingui/react/macro";
import { Command } from "../../lib/commands";
import { ArrowLeftIcon, ArrowRightIcon, CheckIcon } from "@phosphor-icons/react";
import { Page } from "../../components/Page";
import { Banner, Buttons, Chip, QueryError } from "../../components/ui";
import { AssetsStep } from "./AssetsStep";
import { CommitStep } from "./CommitStep";
import { FilePasswordDialog } from "./FilePasswordDialog";
import { FileStep } from "./FileStep";
import { ParseStep } from "./ParseStep";
import { STEPS, fieldLabel, missingFields, readerPluginName } from "./labels";
import { useImportSession } from "./useImportSession";

const LAST = STEPS.length - 1;

export function Import() {
  const { t, i18n } = useLingui();
  const {
    step,
    setStep,
    fileName,
    preview,
    config,
    mapping,
    current,
    overrides,
    template,
    setTemplate,
    options,
    setOptions,
    result,
    sealed,
    setSealed,
    accounts,
    templates,
    plugins,
    load,
    refresh,
    commit,
    apply,
    pickFile,
    applyTemplate,
    reset,
  } = useImportSession();

  const ready = Boolean(config && preview && current);
  const missing = ready ? missingFields(current) : [];

  // The two hard stops of the wizard: rows with no account to land on, and required
  // columns nobody has pointed at — past either of them nothing parses at all.
  const blockedWhy =
    !ready || step !== 1
      ? undefined
      : missing.length > 0
        ? t`Point at these columns first: ${missing.map((field) => fieldLabel(i18n, field)).join(", ")}`
        : !current!.account_id
          ? t`Choose a default account first: otherwise the rows have nowhere to go.`
          : undefined;
  const blocked = blockedWhy !== undefined;
  const reachable = (i: number) => i === 0 || (ready && (i <= step || !blocked));

  return (
    <Page
      archetype="wizard"
      title={t`CSV import`}
      note={fileName ?? undefined}
      lead={i18n._(STEPS[step].hint)}
      steps={STEPS.map(({ title }, i) => (
        <Chip
          key={i}
          active={i === step}
          disabled={!reachable(i)}
          title={i > step && blocked ? blockedWhy : undefined}
          onClick={() => setStep(i)}
        >
          {i < step ? <CheckIcon weight="bold" /> : `${i + 1}.`} {i18n._(title)}
        </Chip>
      ))}
      banner={refresh.isError ? <QueryError error={refresh.error} /> : undefined}
      foot={
        <>
          <Buttons>
            {step > 0 && (
              <button className="btn btn--ghost" onClick={() => setStep(step - 1)}>
                <ArrowLeftIcon /> <Trans>Back</Trans>
              </button>
            )}
            {/* One wording for the whole wizard. It appears only once the file is read:
                until then there is nothing to move on to. */}
            {ready && step < LAST && (
              <button className="btn" disabled={blocked} title={blockedWhy} onClick={() => setStep(step + 1)}>
                <Trans>Next</Trans> <ArrowRightIcon />
              </button>
            )}
            {ready && (
              <button className="btn btn--ghost" onClick={reset}>
                <Trans>Start over</Trans>
              </button>
            )}
          </Buttons>
        </>
      }
    >
      {blocked && <Banner>{blockedWhy}</Banner>}

      <Command id="importFile" run={pickFile} disabled={step !== 0 || load.isPending} />
      {step === 0 && (
        <FileStep
          loading={load.isPending}
          onPick={pickFile}
          // A sealed file is a question for the dialog below, not a failure to show.
          error={sealed ? null : load.error}
          preview={preview}
          config={config}
          mapping={current}
        />
      )}

      {sealed && (
        <FilePasswordDialog
          // A fresh field after a refused password rather than the wrong one left in it.
          key={String(load.submittedAt)}
          plugin={readerPluginName(plugins.data?.plugins, sealed.reader)}
          tried={sealed.tried}
          busy={load.isPending}
          onSubmit={(password) =>
            load.mutate({ path: sealed.path, unlock: { reader: sealed.reader, password } })
          }
          onClose={() => {
            setSealed(null);
            // The password it was sent with must not outlive the dialog in the mutation's state.
            load.reset();
          }}
        />
      )}

      {ready && step === 1 && (
        <ParseStep
          preview={preview!}
          mapping={current!}
          config={config!}
          accounts={accounts.data ?? []}
          templates={templates.data ?? []}
          template={template}
          onTemplate={applyTemplate}
          onForget={() => setTemplate("")}
          onChange={(next) => apply(config!, next, overrides)}
          onConfig={(nextConfig, nextMapping) => apply(nextConfig, nextMapping, overrides)}
        />
      )}

      {ready && step === 2 && (
        <AssetsStep
          preview={preview!}
          mapping={current!}
          onChange={(next) => apply(config!, next, overrides)}
        />
      )}

      {ready && step === 3 && (
        <CommitStep
          preview={preview!}
          overrides={overrides}
          onOverrides={(next) => apply(config!, mapping, next)}
          options={options}
          onOptions={setOptions}
          onCommit={() => commit.mutate()}
          pending={commit.isPending}
          error={commit.error}
          result={result}
        />
      )}
    </Page>
  );
}
