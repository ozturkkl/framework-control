/* eslint-env node */

module.exports = {
	printWidth: 120,
	tabWidth: 4,
	useTabs: true,
	semi: true,
	singleQuote: true,
	trailingComma: 'all',
	bracketSpacing: true,
	arrowParens: 'always',
	endOfLine: 'lf',

	overrides: [
		{
			files: ['*.json', '*.yml', '*.yaml'],
			options: {
				tabWidth: 2,
				useTabs: false,
			},
		},
	],
};
