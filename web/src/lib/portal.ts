// Reparent an overlay to document.body so it is not trapped in a parent stacking context.
export function portal(node: HTMLElement) {
    const previouslyFocused =
        document.activeElement instanceof HTMLElement
            ? document.activeElement
            : null;

    document.body.appendChild(node);

    const dialog =
        node.querySelector<HTMLElement>('[role="dialog"]') ??
        node.querySelector<HTMLElement>(".modal-box") ??
        node;

    requestAnimationFrame(() => {
        dialog.focus();
    });

    return {
        destroy() {
            if (node.parentNode) {
                node.parentNode.removeChild(node);
            }
            previouslyFocused?.focus();
        },
    };
}
