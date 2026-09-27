import { useEffect, type RefObject } from "react";

/** Marks a scrolling box `is-cut` / `is-scrolled` / `is-ended` with classes, not state. */
export function useOverflow(ref: RefObject<HTMLElement | null>, deps: unknown[] = []) {
  useEffect(() => {
    const node = ref.current;
    if (!node) return;

    const read = () => {
      const cut = node.scrollHeight > node.clientHeight + 6;
      node.classList.toggle("is-cut", cut);
      node.classList.toggle("is-scrolled", cut && node.scrollTop > 4);
      node.classList.toggle("is-ended", cut && node.scrollTop + node.clientHeight >= node.scrollHeight - 4);
    };

    read();
    node.addEventListener("scroll", read, { passive: true });
    // The content can change without the box doing so, and the other way round.
    const observer = new ResizeObserver(read);
    observer.observe(node);
    if (node.firstElementChild) observer.observe(node.firstElementChild);
    return () => {
      node.removeEventListener("scroll", read);
      observer.disconnect();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- the caller states what re-measures
  }, deps);
}
