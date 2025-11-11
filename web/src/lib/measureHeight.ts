export type MeasureHeightOptions = {
  onChange: (height: number) => void;
};

// Svelte action to observe an element's height and report updates
export function measureHeight(node: HTMLElement, options: MeasureHeightOptions) {
  let last = -1;
  function report() {
    const h = Math.round(node.clientHeight);
    if (h !== last) {
      last = h;
      options.onChange(h);
    }
  }
  const ro = new ResizeObserver(() => {
    report();
  });
  ro.observe(node);
  // initial measure
  report();
  return {
    destroy() {
      ro.disconnect();
    },
  };
}


