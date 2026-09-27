/** What a conversation is drawn with: the opener, the wait, and each side's words. */

import { Trans, useLingui } from "@lingui/react/macro";
import { SparkleIcon } from "@phosphor-icons/react";
import { Markdown } from "../../ui";

/** A chat with nothing in it yet. The openers are questions this app can actually answer, and
 * they are sent as the user's own words — nothing is prefilled into the model's prompt. */
export function Opener({ onPick }: { onPick: (text: string) => void }) {
  const { t } = useLingui();
  const asks = [
    t`How did my portfolio do this year?`,
    t`What am I most exposed to?`,
    t`Where am I off my target allocation?`,
    t`How much did I pay in fees?`,
  ];
  return (
    <div className="ai-opener">
      <span className="ai-mark ai-mark--big" aria-hidden>
        <SparkleIcon weight="fill" />
      </span>
      <h3>
        <Trans>Ask about your portfolio</Trans>
      </h3>
      <p className="muted">
        <Trans>It reads the figures behind the screen you are on, and asks before it does.</Trans>
      </p>
      <div className="ai-opener__asks">
        {asks.map((ask) => (
          <button key={ask} type="button" className="ai-ask-chip" onClick={() => onPick(ask)}>
            {ask}
          </button>
        ))}
      </div>
    </div>
  );
}

/** The gap between sending and the first thing coming back. Three dots rather than the app's
 * spinner: this is the model taking its time, not the app loading. */
export function Working() {
  const { t } = useLingui();
  return (
    <p className="ai-dots" aria-label={t`Working…`}>
      <span />
      <span />
      <span />
    </p>
  );
}

/** One side's words. The user's sit in a bubble, the model's do not: an answer is the page,
 * not a quote on it. */
export function Said({ role, text, streaming }: { role: string; text: string; streaming?: boolean }) {
  const cls = `ai-said ai-said--${role}${streaming ? " ai-said--streaming" : ""}`;
  return (
    <div className={cls}>
      <Markdown>{text}</Markdown>
    </div>
  );
}
