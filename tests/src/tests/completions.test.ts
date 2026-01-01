import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { cleanupGitRepo, runGitForge, setupGitRepo } from "../utils.js";

describe.each([
    { shell: "bash" } as const,
    { shell: "elvish" } as const,
    { shell: "fish" } as const,
    { shell: "powershell" } as const,
    { shell: "zsh" } as const,
])("Shell completion generation command", ({ shell }) => {
    let tempDir: string;

    beforeEach(() => {
        tempDir = setupGitRepo({ forge: "github" });
    });

    afterEach(() => {
        if (tempDir) {
            cleanupGitRepo(tempDir);
            tempDir = "";
        }
    });

    it("Should generate the completion script for $forge", () => {
        const result = runGitForge({
            args: ["completions", shell],
            cwd: tempDir,
        });

        expect(result.exitCode).toBe(0);
        expect(result.stdout).toBeTruthy();
    });
});
