// DaisyUI requires the base-100 and base-200 keys. base-100 is the low-contrast surface; base-200 is the high-contrast surface used when a panel should pop.

const SHAPE = {
	'--rounded-box': '1rem',
	'--rounded-btn': '0.5rem',
	'--rounded-badge': '1.9rem',
	'--animation-btn': '0.25s',
	'--animation-input': '.2s',
	'--btn-focus-scale': '0.95',
	'--border-btn': '1px',
	'--tab-border': '1px',
	'--tab-radius': '0.5rem',
	'--theme-font-family': 'Inter, ui-sans-serif, system-ui, sans-serif',
};

function radius(value) {
	return {
		'--rounded-box': value,
		'--rounded-btn': value,
		'--rounded-badge': value,
		'--tab-radius': value,
	};
}

const LIGHT_STATES = {
	'info': '#0369A1',
	'info-content': '#FFFFFF',
	'success': '#17B84C',
	'success-content': '#FFFFFF',
	'warning': '#C06D00',
	'warning-content': '#FFFFFF',
	'error': '#CF544B',
	'error-content': '#FFFFFF',
};

const DARK_STATES = {
	'info': '#60A5FA',
	'info-content': '#0C1726',
	'success': '#62EFBD',
	'success-content': '#10251E',
	'warning': '#EFD057',
	'warning-content': '#28220B',
	'error': '#FF8F7D',
	'error-content': '#2B1210',
};

const LIGHT_SENSORS = {
	'--sensor-green': '#1F9D4A',
	'--sensor-blue': '#3B82F6',
	'--sensor-red': '#EF4444',
	'--sensor-purple': '#A855F7',
	'--sensor-amber': '#D4A017',
	'--sensor-emerald': '#10B981',
	'--sensor-orange': '#F97316',
	'--sensor-cyan': '#0891B2',
	'--sensor-yellow': '#EAB308',
	'--sensor-violet': '#8B5CF6',
	'--sensor-pink': '#DB2777',
	'--sensor-teal': '#14B8A6',
};

const DARK_SENSORS = {
	'--sensor-green': '#4ADE80',
	'--sensor-blue': '#60A5FA',
	'--sensor-red': '#FF5C5C',
	'--sensor-purple': '#C084FC',
	'--sensor-amber': '#FBBF24',
	'--sensor-emerald': '#34D399',
	'--sensor-orange': '#FB923C',
	'--sensor-cyan': '#22D3EE',
	'--sensor-yellow': '#FACC15',
	'--sensor-violet': '#A78BFA',
	'--sensor-pink': '#F472B6',
	'--sensor-teal': '#2DD4BF',
};

const THEME_COLORS = {
	'light': {
		'color-scheme': 'light',
		'primary': '#ea7148',
		'primary-content': '#FFFFFF',
		'neutral': '#31363A',
		'neutral-content': '#F5F7F6',
		'base-100': '#ebeaea',
		'base-200': '#f5f4f5',
		'base-content': '#2A2422',
		'--surface-border': '#d1ccc7',
		...LIGHT_STATES,
		...LIGHT_SENSORS,
	},
	'dark': {
		'color-scheme': 'dark',
		'primary': '#FF9D57',
		'primary-content': '#18202A',
		'neutral': '#171B22',
		'neutral-content': '#C4D0D8',
		'base-100': '#2A303C',
		'base-200': '#242933',
		'base-content': '#C4D0D8',
		'--surface-border': '#353D4A',
		...DARK_STATES,
		...DARK_SENSORS,
	},
	'extra-dark': {
		'color-scheme': 'dark',
		'primary': '#E89B52',
		'primary-content': '#17100A',
		'neutral': '#353535',
		'neutral-content': '#CDD3DA',
		'base-100': '#111318',
		'base-200': '#090B0F',
		'base-content': '#CDD3DA',
		'--surface-border': '#252b37',
		...DARK_STATES,
		...DARK_SENSORS,
	},
	'retro': {
		'color-scheme': 'light',
		'primary': '#B36F30',
		'primary-content': '#FFFFFF',
		'neutral': '#6E5944',
		'neutral-content': '#F4EBD4',
		'base-100': '#ECE3C8',
		'base-200': '#F4EBD4',
		'base-content': '#1B1A1A',
		'--surface-border': '#d6c48f',
		...LIGHT_STATES,
		...LIGHT_SENSORS,
		...radius('0.4rem'),
	},
	'synthwave': {
		'color-scheme': 'dark',
		'primary': '#F8C740',
		'primary-content': '#27113F',
		'neutral': '#120A29',
		'neutral-content': '#F6E9FF',
		'base-100': '#2E2058',
		'base-200': '#241945',
		'base-content': '#D8C8E6',
		'--surface-border': '#735CC0',
		...DARK_STATES,
		...DARK_SENSORS,
	},
	'coffee': {
		'color-scheme': 'dark',
		'primary': '#DB924B',
		'primary-content': '#21150D',
		'neutral': '#201B20',
		'neutral-content': '#E8C98C',
		'base-100': '#30232C',
		'base-200': '#291F28',
		'base-content': '#E2C48C',
		'--surface-border': '#141112',
		...DARK_STATES,
		...DARK_SENSORS,
	},
	'terminal': {
		'color-scheme': 'dark',
		'primary': '#4CB66A',
		'primary-content': '#08140C',
		'neutral': '#72C98B',
		'neutral-content': '#08140C',
		'base-100': '#000000',
		'base-200': '#000000',
		'base-content': '#D7DED8',
		'--surface-border': '#22452d',
		...DARK_STATES,
		...DARK_SENSORS,
		...radius('0'),
		'--btn-focus-scale': '1',
		'--theme-font-family':
			'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace',
	},
};

export const FRAMEWORK_THEMES = Object.entries(THEME_COLORS).map(([name, colors]) => ({
	[name]: { ...SHAPE, ...colors },
}));
