import { useEffect } from "react";

const SCROLLING_CLASS = "is-scrolling";
const HIDE_DELAY_MS = 700;

export function useAutoHideScrollbars() {
  useEffect(() => {
    const timers = new Map<Element, number>();

    const markScrolling = (element: Element) => {
      element.classList.add(SCROLLING_CLASS);
      const existingTimer = timers.get(element);
      if (existingTimer) window.clearTimeout(existingTimer);

      const nextTimer = window.setTimeout(() => {
        element.classList.remove(SCROLLING_CLASS);
        timers.delete(element);
      }, HIDE_DELAY_MS);
      timers.set(element, nextTimer);
    };

    const onScroll = (event: Event) => {
      const target =
        event.target === document
          ? document.scrollingElement
          : event.target instanceof Element
            ? event.target
            : null;
      if (!target) return;
      markScrolling(target);
    };

    document.addEventListener("scroll", onScroll, true);
    return () => {
      document.removeEventListener("scroll", onScroll, true);
      for (const [element, timer] of timers) {
        window.clearTimeout(timer);
        element.classList.remove(SCROLLING_CLASS);
      }
      timers.clear();
    };
  }, []);
}
