// Minimal element-based tooltip portal for Svelte
// - Attach the action to the tooltip content element itself
// - We reparent the element to document.body and position it via fixed coords
// - No DaisyUI, no extra wrappers; your element is the tooltip
// - Show/hide is controlled by the `visible` boolean param; anchor is an Element
//
// Usage (example):
// <script lang="ts">
//   import { tooltip } from "$lib/tooltip";
//   let btn: HTMLElement; let tipVisible = false;
// </script>
//
// <button bind:this={btn}
//         on:mouseenter={() => tipVisible = true}
//         on:mouseleave={() => tipVisible = false}>
//   Hover me
// </button>
// <div use:tooltip={{ anchor: btn, visible: tipVisible }}
//      class="rounded shadow px-2 py-1 bg-base-100 border border-base-300 text-sm">
//   Hello tooltip
// </div>
//
// Notes:
// - If no anchor is provided, it defaults to viewport center.
// - Positioning is viewport-aware; it prefers above and flips below when needed, clamping to edges.
// - We only set positioning-related inline styles (position/left/top/maxWidth/zIndex/display/opacity).
// - Everything else (layout/colors/pointer-events) is up to your CSS.

export type TooltipParams = {
  // Anchor element or a function returning an Element
  anchor?: Element | (() => Element | null);
  visible?: boolean; // initial visibility
};

export function tooltip(node: HTMLElement, initial?: TooltipParams) {
  let params: TooltipParams = { ...initial };
  // Tunables (internal constants to keep API minimal)
  const OFFSET = 8;
  const PADDING = 8;
  const Z_INDEX = 10000;

  // Remember original DOM position to restore on destroy
  const originalParent = node.parentElement;
  const originalNextSibling = node.nextSibling;

  // Internal state
  let isShown = false;
  let attached = false;
  let observedAnchor: Element | null = null;
  let mutationObserver: MutationObserver | null = null;

  // Ensure node is under body for clipping-free rendering
  function ensureAttached() {
    if (attached) return;
    try {
      document.body.appendChild(node);
      attached = true;
    } catch {
      // ignore
    }
  }

  function getAnchorRect(): DOMRect {
    const el = resolveAnchorElement();
    if (el) return el.getBoundingClientRect();
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    const w = 1;
    const h = 1;
    const x = Math.max(PADDING, Math.min(vw - PADDING - w, vw / 2 - w / 2));
    const y = Math.max(PADDING, Math.min(vh - PADDING - h, vh / 2 - h / 2));
    // Synthetic rect
    return {
      x,
      y,
      width: w,
      height: h,
      left: x,
      top: y,
      right: x + w,
      bottom: y + h,
      toJSON: () => ({}),
    } as DOMRect;
  }

  // Temporarily reveal for measurement
  function withMeasured<T>(fn: () => T): T {
    const prevDisplay = node.style.display;
    const prevVisibility = node.style.visibility;
    const prevOpacity = node.style.opacity;
    node.style.visibility = "hidden";
    node.style.display = "block";
    node.style.opacity = "0";
    const result = fn();
    // Always restore prior styles so visible tooltips remain visible
    node.style.display = prevDisplay;
    node.style.visibility = prevVisibility;
    node.style.opacity = prevOpacity;
    return result;
  }

  function clamp(val: number, min: number, max: number) {
    return Math.max(min, Math.min(val, max));
  }

  function resolveAnchorElement(): Element | null {
    const a = params.anchor;
    if (typeof a === "function") {
      try {
        const res = a();
        if (res instanceof Element) return res;
        return null;
      } catch {
        return null;
      }
    }
    return a instanceof Element ? a : null;
  }

  function detachAnchorObserver() {
    if (mutationObserver) {
      try {
        mutationObserver.disconnect();
      } catch {}
    }
    mutationObserver = null;
    observedAnchor = null;
  }

  function attachAnchorObserver() {
    const el = resolveAnchorElement();
    if (el === observedAnchor) return;
    detachAnchorObserver();
    if (!el) return;
    observedAnchor = el;
    try {
      mutationObserver = new MutationObserver(() => {
        if (!isShown) return;
        positionNow();
      });
      mutationObserver.observe(el, {
        attributes: true,
        attributeFilter: undefined, // any attribute change (cx, cy, transform, style, etc.)
        subtree: false,
      });
    } catch {
      // ignore observer failures
    }
  }

  function positionNow() {
    if (!attached) return;
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    const pad = PADDING;

    // Prepare base styles
    node.style.position = "fixed";
    node.style.zIndex = String(Z_INDEX);
    node.style.maxWidth = `${vw - pad * 2}px`;

    // Measure tooltip size
    const size = withMeasured(() => node.getBoundingClientRect());
    const anchor = getAnchorRect();
    let left = 0;
    let top = 0;
    const gap = OFFSET;

    // Simple vertical placement: prefer above, otherwise below; clamp to viewport
    const centerX = anchor.left + anchor.width / 2;
    left = Math.round(centerX - size.width / 2);
    left = clamp(left, pad, vw - pad - size.width);
    top = Math.round(anchor.top - size.height - gap);
    if (top < pad) {
      top = Math.min(vh - pad - size.height, anchor.bottom + gap);
    }

    node.style.left = `${left}px`;
    node.style.top = `${top}px`;
  }

  function show() {
    ensureAttached();
    isShown = true;
    node.style.display = "block";
    node.style.opacity = "1";
    attachAnchorObserver();
    positionNow();
  }
  function hide() {
    isShown = false;
    node.style.display = "none";
  }

  // Initial attachment and visibility
  ensureAttached();
  node.style.position = "fixed";
  node.style.left = "0px";
  node.style.top = "0px";
  node.style.zIndex = String(Z_INDEX);
  node.style.display = params.visible ? "block" : "none";
  if (params.visible) {
    // Ensure correct placement on init visible
    attachAnchorObserver();
    positionNow();
  }

  // Keep in place on viewport changes
  const onResize = () => {
    if (isShown) positionNow();
  };
  const onScroll = () => {
    if (isShown) positionNow();
  };
  window.addEventListener("resize", onResize);
  window.addEventListener("scroll", onScroll, true);

  return {
    update(next?: TooltipParams) {
      const wasShown = isShown;
      const prevVisible = params.visible ?? false;
      params = { ...params, ...next };
      const nextVisible = params.visible ?? prevVisible;
      if (next?.visible != null) {
        if (nextVisible && !wasShown) {
          show();
        } else if (!nextVisible && wasShown) {
          hide();
        } else if (nextVisible && wasShown) {
          attachAnchorObserver();
          positionNow();
        }
      } else if (wasShown) {
        attachAnchorObserver();
        positionNow();
      }
    },
    destroy() {
      window.removeEventListener("resize", onResize);
      window.removeEventListener("scroll", onScroll, true);
      detachAnchorObserver();
      // Restore original placement if possible
      if (originalParent) {
        try {
          if (
            originalNextSibling &&
            originalNextSibling.parentNode === originalParent
          ) {
            originalParent.insertBefore(node, originalNextSibling);
          } else {
            originalParent.appendChild(node);
          }
        } catch {
          // ignore
        }
      }
      // Hide on destroy so it does not linger
      node.style.display = "none";
    },
  };
}
