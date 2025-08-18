export function throttleDebounce<Args extends unknown[], R>(
	func: (...args: Args) => R,
	limit: number,
	leading = true,
	trailing = false
): (...args: Args) => R | void {
	let inThrottle = false;
	let trailingCall: (() => R) | null = null;

	return function (this: unknown, ...args: Args): R | void {
		if (!inThrottle) {
			inThrottle = true;
			setTimeout(() => {
				inThrottle = false;
				if (trailing && trailingCall) {
					trailingCall();
					trailingCall = null;
				}
			}, limit);
			if (leading) {
				return func.apply(this, args);
			} else {
				trailingCall = () => func.apply(this, args);
			}
		} else if (trailing) {
			trailingCall = () => func.apply(this, args);
		}
	};
}