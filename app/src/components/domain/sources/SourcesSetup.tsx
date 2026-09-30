import { Trans } from "@lingui/react/macro";
import { Modal } from "../../ui";
import { useSourcesSetup } from "./useSourcesSetup";

/** Asked once, changeable in Settings (ADR-0076). An unpricing set is not sealed; `Decide later` leaves the app saying so. */
export function SourcesSetup({ onClose }: { onClose: () => void }) {
  const { body, actions } = useSourcesSetup({ onDone: onClose });
  return (
    <Modal wide title={<Trans>Where data comes from</Trans>} onClose={onClose} foot={actions}>
      {body}
    </Modal>
  );
}
