import { describe, expect, it } from 'vitest';
import { gtSemver } from './semver';

describe('gtSemver', () => {
	it('compares segments numerically, not as strings', () => {
		expect(gtSemver('0.10.0', '0.9.9')).toBe(true);
		expect(gtSemver('0.9.9', '0.10.0')).toBe(false);
		expect(gtSemver('0.5.5', '0.5.5')).toBe(false);
	});
});
