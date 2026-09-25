import { useMutation } from "@tanstack/react-query";
import { Trans, useLingui } from "@lingui/react/macro";
import { CompassIcon, FlaskIcon } from "@phosphor-icons/react";
import { api } from "../../lib/api";
import { useNav } from "../../lib/nav";
import { useTour } from "../../lib/tour";
import { affects, useInvalidate, useProfiles, useStatus } from "../../lib/queries";
import { openProfile } from "../../lib/profiles";
import { ErrorText, ListRow, Panel } from "../../components/ui";

/** The demo profile is its own, so poking around cannot touch a portfolio that is real. */
const DEMO_PROFILE = "Demo";

/** Learning the app: the tour, and a portfolio to run it over that is nobody's money. */
export function HelpPanel() {
  const { t } = useLingui();
  const tour = useTour();
  const nav = useNav();
  const status = useStatus();
  const profiles = useProfiles();
  const invalidate = useInvalidate();

  const existing = (profiles.data?.profiles ?? []).find((p) => p.name === DEMO_PROFILE);
  const inDemo = existing !== undefined && existing.id === profiles.data?.open;

  // A profile switch reloads the window, so seeding happens in the profile that is already
  // open: this either opens the demo profile, or fills it once it is the open one.
  const demo = useMutation({
    mutationFn: async () => {
      if (inDemo) {
        if ((status.data?.account_count ?? 0) === 0) await api.demoSeed();
        return;
      }
      const profile = existing ?? (await api.profileCreate(DEMO_PROFILE));
      await openProfile(profile.id, profiles.data?.open ?? "");
    },
    onSuccess: () => invalidate(...affects.portfolio),
  });

  return (
    <Panel title={t`Learning the app`} info={t`Where to start, and what to try it on.`}>
      <ListRow
        box
        lead={<CompassIcon />}
        title={t`Guided tour`}
        sub={t`A short walk through what each screen is for. It changes nothing.`}
        end={
          <button
            type="button"
            className="btn btn--sm"
            disabled={tour === null}
            onClick={() => {
              nav.go("dashboard");
              tour?.start();
            }}
          >
            <Trans>Start</Trans>
          </button>
        }
      />
      <ListRow
        box
        lead={<FlaskIcon />}
        title={t`Demo portfolio`}
        sub={
          inDemo
            ? t`This profile is the demo one. Your own data lives in the other profiles.`
            : t`A separate profile filled with three years of made-up history, so nothing you try touches your own.`
        }
        end={
          <button
            type="button"
            className="btn btn--sm"
            disabled={demo.isPending || (inDemo && (status.data?.account_count ?? 0) > 0)}
            onClick={() => demo.mutate()}
          >
            {inDemo ? <Trans>Fill it</Trans> : <Trans>Open it</Trans>}
          </button>
        }
      />
      <ErrorText error={demo.error} />
    </Panel>
  );
}
