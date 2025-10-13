// Minimal Svelte action: keeps DaisyUI tooltip inside viewport by clamping
// the tooltip's horizontal center via CSS variables on the container.
// Usage: <div class="tooltip" data-tip="..."><button use:tooltipClamp>...</button></div>

export function tooltipClamp(node: HTMLElement) {
  const containerMaybe = node.parentElement as HTMLElement | null;
  if (!containerMaybe || !containerMaybe.classList.contains('tooltip')) return {};
  const container: HTMLElement = containerMaybe;

  function measureWidth(text: string): number {
    const probe = document.createElement('div');
    probe.style.position = 'fixed';
    probe.style.visibility = 'hidden';
    probe.style.left = '-9999px';
    probe.style.top = '0';
    probe.style.whiteSpace = 'normal';
    probe.className = 'px-3 py-2 text-sm';
    probe.textContent = text;
    document.body.appendChild(probe);
    const w = probe.getBoundingClientRect().width;
    probe.remove();
    return Math.ceil(w);
  }

  function updateClamp() {
    const text = container.getAttribute('data-tip') || '';
    if (!text) {
      container.classList.remove('tooltip-clamped');
      container.style.removeProperty('--tc-left');
      container.style.removeProperty('--tc-translate-x');
      return;
    }
    const rect = container.getBoundingClientRect();
    const vw = window.innerWidth;
    const centerX = rect.left + rect.width / 2;
    const width = measureWidth(text);
    const half = width / 2; // no extra slack; allow bubble to sit flush
    const margin = 0;
    const minCenter = margin + half;
    const maxCenter = Math.max(minCenter, vw - margin - half);
    const clampedCenter = Math.min(Math.max(centerX, minCenter), maxCenter);
    const delta = clampedCenter - centerX; // pixels to nudge horizontally
    container.classList.add('tooltip-clamped');
    container.style.setProperty('--tc-ml', `${delta}px`);
  }

  const onEnter = () => updateClamp();
  const onResize = () => updateClamp();
  const onScroll = () => updateClamp();
  node.addEventListener('mouseenter', onEnter);
  node.addEventListener('focus', onEnter);
  window.addEventListener('resize', onResize);
  window.addEventListener('scroll', onScroll, true);

  return {
    destroy() {
      node.removeEventListener('mouseenter', onEnter);
      node.removeEventListener('focus', onEnter);
      window.removeEventListener('resize', onResize);
      window.removeEventListener('scroll', onScroll, true);
      container.classList.remove('tooltip-clamped');
      container.style.removeProperty('--tc-ml');
    }
  };
}


