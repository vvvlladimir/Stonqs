import { createContext, useContext, useState, type ReactNode } from "react";

/** The dock's portal target, handed over by the element itself; a lookup by id is a render late. */
const DockContext = createContext<{
  slot: HTMLElement | null;
  hold: (node: HTMLElement | null) => void;
}>({ slot: null, hold: () => {} });

export function DockProvider({ children }: { children: ReactNode }) {
  const [slot, setSlot] = useState<HTMLElement | null>(null);
  return <DockContext.Provider value={{ slot, hold: setSlot }}>{children}</DockContext.Provider>;
}

/** The element screens portal into; rendered once, by the shell. */
export function DockSlot() {
  const { hold } = useContext(DockContext);
  return <div className="dock__slot" ref={hold} />;
}

/** The target itself, or `null` until the shell has drawn it. */
export function useDockSlot(): HTMLElement | null {
  return useContext(DockContext).slot;
}
