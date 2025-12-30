import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { cleanupGitRepo, runGitForge, setupGitRepo } from "../utils.js";

describe("Web Command", () => {
    let tempDir: string;

    afterEach(() => {
        if (tempDir) {
            cleanupGitRepo(tempDir);
            tempDir = "";
        }
    });

    describe("GitHub", () => {
        beforeEach(() => {
            tempDir = setupGitRepo({ forge: "github" });
        });

        it("Should display help", () => {
            const result = runGitForge({
                args: ["web", "--help"],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBeTruthy();
        });

        it("Should generate web URL using full 'web' command", () => {
            const result = runGitForge({
                args: ["web", "--api", "github"],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBe("https://localhost:3001/user/repo");
        });

        it("Should generate web URL using 'w' alias", () => {
            const result = runGitForge({
                args: ["w", "--api", "github"],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBeTruthy();
        });

        it("Should generate web URL for GitHub issues", () => {
            const result = runGitForge({
                args: ["web", "--api", "github", "--target", "issues"],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBe(
                "https://localhost:3001/user/repo/issues",
            );
        });

        it("Should generate web URL for GitHub prs", () => {
            const result = runGitForge({
                args: ["web", "--api", "github", "--target", "prs"],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBe(
                "https://localhost:3001/user/repo/pulls",
            );
        });
    });

    describe("GitLab", () => {
        beforeEach(() => {
            tempDir = setupGitRepo({ forge: "gitlab" });
        });

        it("Should generate web URL for GitLab repository", () => {
            const result = runGitForge({
                args: ["web", "--api", "gitlab"],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBe("https://localhost:3002/user/repo");
        });

        it("Should generate web URL for GitLab issues", () => {
            const result = runGitForge({
                args: ["web", "--api", "gitlab", "--target", "issues"],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBe(
                "https://localhost:3002/user/repo/-/issues",
            );
        });

        it("Should generate web URL for GitLab merge requests", () => {
            const result = runGitForge({
                args: ["web", "--api", "gitlab", "--target", "mrs"],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBe(
                "https://localhost:3002/user/repo/-/merge_requests",
            );
        });
    });

    describe("Gitea", () => {
        beforeEach(() => {
            tempDir = setupGitRepo({ forge: "gitea" });
        });

        it("Should generate web URL for Gitea repository", () => {
            const result = runGitForge({
                args: ["web", "--api", "gitea"],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBe("https://localhost:3003/user/repo");
        });

        it("Should generate web URL for Gitea issues", () => {
            const result = runGitForge({
                args: ["web", "--api", "gitea", "--target", "issues"],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBe(
                "https://localhost:3003/user/repo/issues",
            );
        });

        it("Should generate web URL for Gitea pull requests", () => {
            const result = runGitForge({
                args: ["web", "--api", "gitea", "--target", "prs"],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBe(
                "https://localhost:3003/user/repo/pulls",
            );
        });
    });
});
