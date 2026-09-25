import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { api } from "../lib/api";
import { affects, useInvalidate, useProfiles } from "../lib/queries";
import { openProfile } from "../lib/profiles";
import { ProfileRows } from "../components/domain/ProfilePicker";
import { ErrorText, Field, FieldPair, Form, Gate, GateSection } from "../components/ui";
import type { AppStatus, SetupInput } from "../lib/types";

export function Onboarding({ status }: { status: AppStatus }) {
  const { t } = useLingui();
  const invalidate = useInvalidate();
  const [draft, setDraft] = useState<SetupInput>({
    portfolio_name: status.portfolio_name,
    base_currency: status.base_currency,
    account_name: "",
    account_currency: status.base_currency,
    securities_account_name: "",
  });

  const setup = useMutation({
    mutationFn: api.setupPortfolio,
    onSuccess: () => invalidate(...affects.portfolio),
  });

  // A new profile opens here, empty; the way back to the others must not require setting it up.
  const profiles = useProfiles();
  const others = (profiles.data?.profiles ?? []).filter((p) => p.id !== profiles.data?.open);
  const leave = useMutation({ mutationFn: (id: string) => openProfile(id, profiles.data?.open ?? "") });

  const seed = useMutation({
    mutationFn: api.demoSeed,
    onSuccess: () => invalidate(...affects.portfolio),
  });

  const currency = (value: string) => value.toUpperCase();

  return (
    <Gate
      title={<Trans>Let's set up the portfolio</Trans>}
      lead={t`Money and instruments live on separate accounts: cash on one, securities on another that settles through it.`}
      foot={
        <>
          {others.length > 0 && (
            <GateSection title={t`Other profiles`}>
              <ProfileRows profiles={others} disabled={leave.isPending} onPick={(id) => leave.mutate(id)} />
              <ErrorText error={leave.error} />
            </GateSection>
          )}
          <GateSection title={t`Just looking`}>
            <p className="dim">
              <Trans>
                Fills this profile with a sample portfolio and offers a short tour, so you can see what the
                app does before importing anything of your own.
              </Trans>
            </p>
            <button className="btn btn--ghost" onClick={() => seed.mutate()} disabled={seed.isPending}>
              {seed.isPending ? t`Filling…` : t`Try with a demo portfolio`}
            </button>
            <ErrorText error={seed.error} />
          </GateSection>
        </>
      }
    >
      <Form
        onSubmit={() => setup.mutate(draft)}
        busy={setup.isPending}
        error={setup.error}
        ready={
          draft.portfolio_name.trim() !== "" &&
          draft.account_name.trim() !== "" &&
          draft.base_currency.length === 3 &&
          draft.account_currency.length === 3
        }
        submitLabel={t`Create portfolio`}
        busyLabel={t`Creating…`}
      >
        <FieldPair>
          <Field label={t`Portfolio name`}>
            <input
              value={draft.portfolio_name}
              onChange={(e) => setDraft({ ...draft, portfolio_name: e.target.value })}
            />
          </Field>
          <Field label={t`Base currency`}>
            <input
              value={draft.base_currency}
              maxLength={3}
              onChange={(e) => setDraft({ ...draft, base_currency: currency(e.target.value) })}
            />
          </Field>
        </FieldPair>

        <FieldPair>
          <Field label={t`Cash account`} hint={t`Money sits here: deposits, fees, interest`}>
            <input
              autoFocus
              value={draft.account_name}
              placeholder={t`Main · Broker`}
              onChange={(e) => setDraft({ ...draft, account_name: e.target.value })}
            />
          </Field>
          <Field label={t`Currency`}>
            <input
              value={draft.account_currency}
              maxLength={3}
              onChange={(e) => setDraft({ ...draft, account_currency: currency(e.target.value) })}
            />
          </Field>
        </FieldPair>

        <Field label={t`Securities account`} hint={t`Optional: it can be created later.`}>
          <input
            value={draft.securities_account_name ?? ""}
            placeholder={t`Trade Republic`}
            onChange={(e) => setDraft({ ...draft, securities_account_name: e.target.value })}
          />
        </Field>
      </Form>
    </Gate>
  );
}
