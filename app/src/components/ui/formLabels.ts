import { msg } from "@lingui/core/macro";

/**
 * The wording of a form's buttons, written once: a screen never writes "Saving…" itself, and
 * passes a verb of its own only when "Save" is the wrong one.
 */
export const SUBMIT = msg`Save`;
export const SUBMITTING = msg`Saving…`;
export const CANCEL = msg`Cancel`;
