export function listAvailableDaisyUIThemes(doc: Document = document): string[] {
  const seen = new Set<string>();

  const tryExtract = (selector: string) => {
    // Matches [data-theme="name"] or [data-theme=name]
    const re = /\[data-theme=(?:"|')?([a-z0-9-]+)(?:"|')?\]/gi;
    let m: RegExpExecArray | null;
    while ((m = re.exec(selector)) !== null) {
      if (m[1]) seen.add(m[1]);
    }
  };

  const walkRules = (cssRules: CSSRuleList) => {
    if (!cssRules) return;
    for (let i = 0; i < cssRules.length; i++) {
      const rule = cssRules[i];
      try {
        if ((rule as CSSStyleRule).selectorText) {
          tryExtract((rule as CSSStyleRule).selectorText);
        }
      } catch {
        // ignore parse/security errors per rule
      }
    }
  };

  for (const sheet of Array.from(doc.styleSheets)) {
    try {
      walkRules(sheet.cssRules);
    } catch {
      // Cross-origin or inaccessible stylesheet; skip
    }
  }

  // Ensure common defaults at the front if present
  const ordered = Array.from(seen);
  const promote = (name: string) => {
    const idx = ordered.indexOf(name);
    if (idx > 0) {
      ordered.splice(idx, 1);
      ordered.unshift(name);
    }
  };
  promote("dark");
  promote("light");

  return ordered;
}
