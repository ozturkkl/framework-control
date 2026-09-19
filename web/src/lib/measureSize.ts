export type MeasuredSize = {
	width: number;
	height: number;
};

export type MeasureSizeOptions = {
	onChange: (size: MeasuredSize) => void;
};

// ResizeObserver on the content box; reports integer clientWidth/clientHeight together.
export function measureSize(node: HTMLElement, options: MeasureSizeOptions) {
	let lastW = -1;
	let lastH = -1;

	function report() {
		const width = Math.round(node.clientWidth);
		const height = Math.round(node.clientHeight);
		if (width <= 0 || height <= 0) return;
		if (width === lastW && height === lastH) return;
		lastW = width;
		lastH = height;
		options.onChange({ width, height });
	}

	const ro = new ResizeObserver(() => {
		report();
	});
	ro.observe(node, { box: 'content-box' });
	report();

	return {
		update(next: MeasureSizeOptions) {
			options = next;
		},
		destroy() {
			ro.disconnect();
		},
	};
}
