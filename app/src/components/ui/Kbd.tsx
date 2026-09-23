import { Fragment } from "react";

/**
 * A key combination as printed on the keyboard. Takes what `keyParts` returns: one group per
 * step of a sequence, one cap per key of a chord.
 */
export function Kbd({ steps }: { steps: string[][] }) {
  return (
    <span className="kbd">
      {steps.map((keys, i) => (
        <Fragment key={i}>
          {i > 0 && <span className="kbd__then" aria-hidden />}
          {keys.map((key, j) => (
            <kbd key={j}>{key}</kbd>
          ))}
        </Fragment>
      ))}
    </span>
  );
}
