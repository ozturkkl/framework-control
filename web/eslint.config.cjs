/* eslint-env node */
const tsParser = require('@typescript-eslint/parser');
const tsPlugin = require('@typescript-eslint/eslint-plugin');
const svelteParser = require('svelte-eslint-parser');
const sveltePlugin = require('eslint-plugin-svelte');

module.exports = [
	{ ignores: ['dist/**', 'build/**', '.svelte-kit/**', 'node_modules/**', 'src/api/**'] },
	{
		files: ['**/*.ts'],
		languageOptions: {
			parser: tsParser,
			ecmaVersion: 'latest',
			sourceType: 'module',
		},
		plugins: { '@typescript-eslint': tsPlugin },
		rules: {
			'@typescript-eslint/no-explicit-any': 'error',
		},
	},
	{
		files: ['**/*.svelte'],
		languageOptions: {
			parser: svelteParser,
			parserOptions: {
				parser: tsParser,
				ecmaVersion: 'latest',
				sourceType: 'module',
				extraFileExtensions: ['.svelte'],
			},
		},
		plugins: { svelte: sveltePlugin, '@typescript-eslint': tsPlugin },
		rules: {
			'@typescript-eslint/no-explicit-any': 'error',
		},
	},
];


