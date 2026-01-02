import { afterEach, beforeEach, describe, expect, it } from "vitest";

import {
    cleanupGitRepo,
    getApiUrl,
    runGitForge,
    setupGitRepo,
} from "../utils.js";

describe.each([
    { forge: "github", token: "GITHUB_TOKEN" } as const,
    { forge: "gitea", token: "GITEA_TOKEN" } as const,
    { forge: "gitlab", token: "GITLAB_TOKEN" } as const,
])("Issue Create Command ($forge)", ({ forge, token }) => {
    let localRepoDir: string;

    beforeEach(() => {
        localRepoDir = setupGitRepo({ forge });
    });

    afterEach(() => {
        if (localRepoDir) {
            cleanupGitRepo(localRepoDir);
            localRepoDir = "";
        }
    });

    it("Should display help", () => {
        const result = runGitForge({
            args: ["issue", "create", "--help"],
            cwd: localRepoDir,
        });

        expect(result.exitCode).toBe(0);
        expect(result.stdout).toBeTruthy();
    });

    it("Should work with alias 'cr'", () => {
        const result = runGitForge({
            args: ["issue", "cr", "--help"],
            cwd: localRepoDir,
        });

        expect(result.exitCode).toBe(0);
        expect(result.stdout).toBeTruthy();
    });

    it("Should create issue with title only", () => {
        const result = runGitForge({
            args: [
                "issue",
                "create",
                "--no-browser",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--title",
                "Test Issue",
            ],
            cwd: localRepoDir,
            env: { [token]: "test-token" },
        });

        expect(result.exitCode).toBe(0);
        expect(result.stdout).toContain("Issue created at");
    });

    it("Should create issue with title and body", () => {
        const result = runGitForge({
            args: [
                "issue",
                "create",
                "--no-browser",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--title",
                "Issue with Body",
                "--body",
                "This is the issue description",
            ],
            cwd: localRepoDir,
            env: { [token]: "test-token" },
        });

        expect(result.exitCode).toBe(0);
        expect(result.stdout).toContain("Issue created at");
    });

    it("Should fail when creating issue without authentication", () => {
        const result = runGitForge({
            args: [
                "issue",
                "create",
                "--no-browser",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--title",
                "Test Issue",
            ],
            cwd: localRepoDir,
            throwsError: true,
        });

        expect(result.exitCode).not.toBe(0);
        expect(result.stderr).toBeTruthy();
    });

    it("Should fail when creating issue without title", () => {
        const result = runGitForge({
            args: [
                "issue",
                "create",
                "--no-browser",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
            ],
            cwd: localRepoDir,
            env: { [token]: "test-token" },
            throwsError: true,
        });

        expect(result.exitCode).not.toBe(0);
        expect(result.stderr).toBeTruthy();
    });
});
