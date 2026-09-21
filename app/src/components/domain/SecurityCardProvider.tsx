import { createContext, useContext, useState, type ReactNode } from "react";
import { SecurityCard } from "./SecurityPopup";

/**
 * One instrument card for the whole app: a ticker is printed on nearly every screen, and none of
 * them should have to own a modal to make it clickable. The provider holds the id, the card reads
 * everything else itself.
 */
const SecurityCardContext = createContext<{ open: (securityId: string) => void }>({ open: () => {} });

export function SecurityCardProvider({ children }: { children: ReactNode }) {
  const [securityId, setSecurityId] = useState<string | null>(null);
  return (
    <SecurityCardContext.Provider value={{ open: setSecurityId }}>
      {children}
      {securityId && <SecurityCard securityId={securityId} onClose={() => setSecurityId(null)} />}
    </SecurityCardContext.Provider>
  );
}

export function useSecurityCard() {
  return useContext(SecurityCardContext);
}

/**
 * Wraps whatever prints an instrument — a ticker, a name, an ISIN — so one click opens its card.
 * Without an id there is nothing to open, and the text is rendered as it was: an import preview
 * names instruments that do not exist yet.
 */
export function SecurityLink({
  id,
  className,
  children,
}: {
  id?: string | null;
  className?: string;
  children: ReactNode;
}) {
  const card = useSecurityCard();
  // Without an id the element still has to carry the caller's layout class, so it is a span,
  // not a bare fragment.
  if (!id) return <span className={className}>{children}</span>;
  return (
    <button
      type="button"
      className={className ? `linkrow ${className}` : "linkrow"}
      onClick={(e) => {
        // A row may open a menu or a dialog of its own; the card is what this click asked for.
        e.stopPropagation();
        card.open(id);
      }}
    >
      {children}
    </button>
  );
}
