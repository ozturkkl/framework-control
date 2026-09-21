const SYSTEM_THEME = 'system' as const;

const THEME_OPTIONS = [
	{ value: SYSTEM_THEME, label: 'System' },
	{ value: 'dark', label: 'Dark' },
	{ value: 'extra-dark', label: 'Extra Dark' },
	{ value: 'light', label: 'Light' },
	{ value: 'retro', label: 'Retro' },
	{ value: 'synthwave', label: 'Synthwave' },
	{ value: 'coffee', label: 'Coffee' },
	{ value: 'terminal', label: 'Terminal' },
] as const;

type ThemePreference = (typeof THEME_OPTIONS)[number]['value'];
type ConcreteTheme = Exclude<ThemePreference, typeof SYSTEM_THEME>;

const THEME_PREFERENCES = new Set<string>(THEME_OPTIONS.map(({ value }) => value));
let currentTheme: ThemePreference = SYSTEM_THEME;
let systemTheme: MediaQueryList | null = null;
let listeningForSystemTheme = false;

function normalize(theme: string | null | undefined): ThemePreference {
	return theme && THEME_PREFERENCES.has(theme) ? (theme as ThemePreference) : SYSTEM_THEME;
}

function resolve(theme: string | null | undefined, prefersDark: boolean): ConcreteTheme {
	const preference = normalize(theme);
	if (preference === SYSTEM_THEME) {
		return prefersDark ? 'dark' : 'light';
	}
	return preference;
}

function getSystemTheme(): MediaQueryList {
	return (systemTheme ??= window.matchMedia('(prefers-color-scheme: dark)'));
}

function apply(theme: string | null | undefined): ThemePreference {
	currentTheme = normalize(theme);
	document.documentElement.setAttribute('data-theme', resolve(currentTheme, getSystemTheme().matches));
	return currentTheme;
}

function initialize(theme: string | null | undefined): ThemePreference {
	if (!listeningForSystemTheme) {
		getSystemTheme().addEventListener('change', () => {
			if (currentTheme === SYSTEM_THEME) apply(currentTheme);
		});
		listeningForSystemTheme = true;
	}
	return apply(theme);
}

export const themes = {
	get current(): ThemePreference {
		return currentTheme;
	},
	options: THEME_OPTIONS,
	normalize,
	apply,
	initialize,
};
