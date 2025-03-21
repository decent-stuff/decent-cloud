import js from '@eslint/js';
import tseslint from 'typescript-eslint';
import nextPlugin from '@next/eslint-plugin-next';
import promisePlugin from 'eslint-plugin-promise';
import globals from 'globals';

export default [
  js.configs.recommended,
  ...tseslint.configs.recommended,
  // TypeScript files configuration
  {
    files: ['**/*.ts', '**/*.tsx'],
    plugins: {
      '@next/next': nextPlugin,
    },
    languageOptions: {
      parser: tseslint.parser,
      parserOptions: {
        project: './tsconfig.json',
        ecmaVersion: 2020,
        sourceType: 'module',
      },
      globals: {
        ...globals.browser,
        ...globals.node,
      },
    },
    rules: {
      '@typescript-eslint/await-thenable': 'error',
      '@typescript-eslint/no-floating-promises': 'error',
      'no-void': ['error', { allowAsStatement: true }],
    },
  },
  // JavaScript files configuration
  {
    files: ['**/*.js', '**/*.mjs'],
    plugins: {
      '@next/next': nextPlugin,
      promise: promisePlugin,
    },
    languageOptions: {
      ecmaVersion: 2020,
      sourceType: 'module',
      globals: {
        ...globals.browser,
        ...globals.node,
      },
    },
    rules: {
      'require-await': 'error',
      'no-void': ['error', { allowAsStatement: true }],
      'no-promise-executor-return': 'error',
      'promise/catch-or-return': 'error',
      'promise/always-return': 'error',
    },
  },
  // Ignore dist and node_modules directories
  {
    ignores: ['dist/**', 'node_modules/**', 'demo/dist/**', 'demo/node_modules/**'],
  },
];
