import { Fragment } from "react";

/** A key combination from `keyParts`: one group per step, one cap per key. */
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
