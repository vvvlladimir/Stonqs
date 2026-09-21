import { useEffect, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { XIcon } from "@phosphor-icons/react";
import { useLingui } from "@lingui/react/macro";

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
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [onClose]);

  return createPortal(
    <div className="backdrop" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className={`modal${wide ? " modal--wide" : ""}`} role="dialog" aria-modal="true">
        <div className="modal__head">
          <h2>{title}</h2>
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
