import { useEffect, useRef } from "react";

/** Follows the conversation while it is at the bottom, and stops following the moment the user
 * scrolls up to read something — an answer being read must not be yanked away by the next token. */
export function useScrollToEnd(deps: unknown[]) {
  const ref = useRef<HTMLDivElement>(null);
  const stick = useRef(true);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const onScroll = () => {
      stick.current = el.scrollHeight - el.scrollTop - el.clientHeight < 80;
    };
    el.addEventListener("scroll", onScroll, { passive: true });
    return () => el.removeEventListener("scroll", onScroll);
  }, []);

  useEffect(() => {
    const el = ref.current;
    if (el && stick.current) el.scrollTop = el.scrollHeight;
    // The conversation's own length and the streaming text: what changed is what to follow.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);

  return ref;
}
