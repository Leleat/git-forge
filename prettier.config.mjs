/** @type {import("prettier").Config} */
export default {
    tabWidth: 4,
    experimentalTernaries: true,
    overrides: [
        {
            files: ["*.json", "*.json5", "*.jsonc"],
            options: {
                tabWidth: 2,
            },
        },
    ],
};
