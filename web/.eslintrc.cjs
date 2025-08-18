/* eslint-env node */
module.exports = {
	root: true,
	overrides: [
		{
			files: ['*.svelte'],
			parser: 'svelte-eslint-parser',
			parserOptions: {
				ecmaVersion: 'latest',
				sourceType: 'module',
				extraFileExtensions: ['.svelte'],
				parser: '@typescript-eslint/parser',
				project: false,
			},
			extends: [
				'plugin:svelte/recommended',
				'plugin:@typescript-eslint/recommended',
			],
			rules: {
				'@typescript-eslint/no-explicit-any': 'error',
			},
		},
		{
			files: ['*.ts'],
			parser: '@typescript-eslint/parser',
			parserOptions: {
				ecmaVersion: 'latest',
				sourceType: 'module',
				project: false,
			},
			extends: [
				'plugin:@typescript-eslint/recommended',
			],
			rules: {
				'@typescript-eslint/no-explicit-any': 'error',
			},
		},
	],
};


