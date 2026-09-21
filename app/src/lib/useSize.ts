import { useEffect, useRef, useState } from "react";

/**
 * Container size in real pixels, not a stretched `viewBox` — that would
 * scale axis labels down with the geometry on narrow screens. The height is
 * what a chart asked to fill its tile measures itself against.
 */
export function useSize<T extends HTMLElement>() {
  const ref = useRef<T>(null);
  const [size, setSize] = useState({ width: 0, height: 0 });

  useEffect(() => {
    const node = ref.current;
    if (!node) return;
    const observer = new ResizeObserver(([entry]) =>
      setSize({ width: entry.contentRect.width, height: entry.contentRect.height }),
    );
    observer.observe(node);
    return () => observer.disconnect();
  }, []);

  return { ref, width: size.width, height: size.height };
}
