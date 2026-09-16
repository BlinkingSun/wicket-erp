import js from "@eslint/js";
import globals from "globals";
import reactHooks from "eslint-plugin-react-hooks";
import reactRefresh from "eslint-plugin-react-refresh";
import tseslint from "typescript-eslint";

const httpBelongsInClient =
  "HTTP belongs in src/api/client.ts. A component that fetches bypasses the T-35 swap point.";

const typesMustNotLeak =
  "Throwaway API types in src/api/types/ must not leak into components. Map to a view-model inside src/api/.";

const clientMustNotReexport =
  "client.ts must not re-export throwaway types from src/api/types/. Re-exporting launders the leak through an allowed module.";

export default tseslint.config(
  { ignores: ["dist", "node_modules"] },
  {
    extends: [js.configs.recommended, ...tseslint.configs.recommended],
    files: ["**/*.{ts,tsx}"],
    languageOptions: {
      ecmaVersion: 2022,
      globals: globals.browser,
    },
    plugins: {
      "react-hooks": reactHooks,
      "react-refresh": reactRefresh,
    },
    rules: {
      ...reactHooks.configs.recommended.rules,
      "react-refresh/only-export-components": [
        "warn",
        { allowConstantExport: true },
      ],
    },
  },
  {
    files: ["src/**/*.{ts,tsx}"],
    ignores: ["src/api/**"],
    rules: {
      "no-restricted-imports": [
        "error",
        {
          paths: [
            {
              name: "axios",
              message: httpBelongsInClient,
            },
            {
              name: "ky",
              message: httpBelongsInClient,
            },
            {
              name: "node-fetch",
              message: httpBelongsInClient,
            },
          ],
          patterns: [
            {
              group: [
                "**/api/types",
                "**/api/types/*",
                "**/api/types/**",
                "@/api/types",
                "@/api/types/*",
                "@/api/types/**",
              ],
              message: typesMustNotLeak,
            },
          ],
        },
      ],
      "no-restricted-globals": [
        "error",
        { name: "fetch", message: httpBelongsInClient },
      ],
      "no-restricted-properties": [
        "error",
        {
          object: "window",
          property: "fetch",
          message: httpBelongsInClient,
        },
        {
          object: "globalThis",
          property: "fetch",
          message: httpBelongsInClient,
        },
      ],
      "no-restricted-syntax": [
        "error",
        {
          selector: "CallExpression[callee.name='fetch']",
          message: httpBelongsInClient,
        },
        {
          selector:
            "CallExpression[callee.object.name='window'][callee.property.name='fetch']",
          message: httpBelongsInClient,
        },
        {
          selector:
            "CallExpression[callee.object.name='globalThis'][callee.property.name='fetch']",
          message: httpBelongsInClient,
        },
        {
          selector:
            "MemberExpression[object.name='window'][property.name='fetch']",
          message: httpBelongsInClient,
        },
        {
          selector:
            "MemberExpression[object.name='globalThis'][property.name='fetch']",
          message: httpBelongsInClient,
        },
      ],
    },
  },
  {
    files: ["src/api/client.ts"],
    rules: {
      "no-restricted-syntax": [
        "error",
        {
          selector:
            "ExportNamedDeclaration[source.value=/types/]",
          message: clientMustNotReexport,
        },
        {
          selector:
            "ExportAllDeclaration[source.value=/types/]",
          message: clientMustNotReexport,
        },
      ],
    },
  },
);
