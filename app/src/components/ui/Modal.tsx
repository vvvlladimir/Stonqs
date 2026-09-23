import { useId, useRef, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { XIcon } from "@phosphor-icons/react";
import { useLingui } from "@lingui/react/macro";
import { useLayer } from "../../lib/shortcuts";
import { useDialogFocus } from "./focus";

/** Responsive modal rendered in a body portal so ancestor overflow cannot clip it. */
interface Props {
  title: ReactNode;
  onClose: () => void;
  /** Optional decision controls rendered below the modal content. */
  foot?: ReactNode;
  wide?: boolean;
  children: ReactNode;
}

export function Modal({ title, onClose, foot, wide, children }: Props) {
  const { t } = useLingui();
  const box = useRef<HTMLDivElement>(null);
  const heading = useId();
  // One `Escape` closes the top dialog only, not every layer under it.
  useLayer(onClose);
  useDialogFocus(box);

  return createPortal(
    <div className="backdrop" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div
        ref={box}
        className={`modal${wide ? " modal--wide" : ""}`}
        role="dialog"
        aria-modal="true"
        aria-labelledby={heading}
        tabIndex={-1}
      >
        <div className="modal__head">
          <h2 id={heading}>{title}</h2>
          <span className="spacer" />
          <button type="button" className="iconbtn" aria-label={t`Close`} onClick={onClose}>
            <XIcon />
          </button>
        </div>
        {children}
        {foot && <div className="modal__foot">{foot}</div>}
      </div>
    </div>,
    document.body,
  );
}
