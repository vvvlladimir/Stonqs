import { useId, type ReactNode } from "react";
import { useLingui } from "@lingui/react/macro";
import { Modal } from "./Modal";
import { ErrorText, Submit } from "./Form";
import { CANCEL, SUBMIT, SUBMITTING } from "./formLabels";
import { IS_MAC } from "../../lib/commands";

export interface FormDialogProps {
  title: ReactNode;
  onClose: () => void;
  onSubmit: () => void;
  busy?: boolean;
  error?: Error | null;
  submitLabel?: string;
  busyLabel?: string;
  /** Blocks submit while false: an empty name, an excluded subject, a sum over 100 %. */
  ready?: boolean;
  wide?: boolean;
  /** A button of its own meaning (delete, disable), pinned to the left of the footer. */
  lead?: ReactNode;
  children: ReactNode;
}

/**
 * Modal + form. The submit button sits in the modal footer but belongs to the form through
 * `form=<id>`, so Enter in any field saves and the footer stays below the scrolling body.
 */
export function FormDialog({
  title,
  onClose,
  onSubmit,
  busy,
  error,
  submitLabel,
  busyLabel,
  ready,
  wide,
  lead,
  children,
}: FormDialogProps) {
  const { i18n } = useLingui();
  const id = useId();

  return (
    <Modal
      title={title}
      onClose={onClose}
      wide={wide}
      foot={
        <>
          {lead}
          <span className="spacer" />
          <button type="button" className="btn btn--ghost" onClick={onClose}>
            {i18n._(CANCEL)}
          </button>
          <Submit
            form={id}
            busy={busy}
            ready={ready}
            label={submitLabel ?? i18n._(SUBMIT)}
            busyLabel={busyLabel ?? i18n._(SUBMITTING)}
          />
        </>
      }
    >
      <form
        id={id}
        className="form"
        onSubmit={(e) => {
          e.preventDefault();
          onSubmit();
        }}
        // `mod+Enter` saves from anywhere in the form, a multi-line note included, where Enter
        // is a new line. It asks what the Save button would: nothing while blocked or busy.
        onKeyDown={(e) => {
          if (e.key !== "Enter" || !(IS_MAC ? e.metaKey : e.ctrlKey) || e.nativeEvent.isComposing) return;
          e.preventDefault();
          if (ready !== false && !busy) e.currentTarget.requestSubmit();
        }}
      >
        {children}
        <ErrorText error={error} />
      </form>
    </Modal>
  );
}
