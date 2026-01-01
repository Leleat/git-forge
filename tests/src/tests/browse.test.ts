import { execSync } from "node:child_process";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { cleanupGitRepo, runGitForge, setupGitRepo } from "../utils.js";

describe("Browse Command", () => {
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
                args: ["browse", "--help"],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBeTruthy();
        });

        it("Should generate repository home URL", () => {
            const result = runGitForge({
                args: ["browse", "--no-browser", "--api", "github"],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBe("https://localhost:3001/user/repo");
        });

        it("Should generate issues list URL with --issues flag", () => {
            const result = runGitForge({
                args: ["browse", "--no-browser", "--api", "github", "--issues"],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBe(
                "https://localhost:3001/user/repo/issues",
            );
        });

        it("Should generate issues list URL with -i flag", () => {
            const result = runGitForge({
                args: ["browse", "--no-browser", "--api", "github", "-i"],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBe(
                "https://localhost:3001/user/repo/issues",
            );
        });

        it("Should generate specific issue URL", () => {
            const result = runGitForge({
                args: [
                    "browse",
                    "--no-browser",
                    "--api",
                    "github",
                    "-i",
                    "123",
                ],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBe(
                "https://localhost:3001/user/repo/issues/123",
            );
        });

        it("Should generate PRs list URL with --prs flag", () => {
            const result = runGitForge({
                args: ["browse", "--no-browser", "--api", "github", "--prs"],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBe(
                "https://localhost:3001/user/repo/pulls",
            );
        });

        it("Should generate PRs list URL with -p flag", () => {
            const result = runGitForge({
                args: ["browse", "--no-browser", "--api", "github", "-p"],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBe(
                "https://localhost:3001/user/repo/pulls",
            );
        });

        it("Should generate specific PR URL", () => {
            const result = runGitForge({
                args: ["browse", "--no-browser", "--api", "github", "-p", "42"],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBe(
                "https://localhost:3001/user/repo/pull/42",
            );
        });

        it("Should generate commit URL with --commit flag", () => {
            const result = runGitForge({
                args: [
                    "browse",
                    "--no-browser",
                    "--api",
                    "github",
                    "--commit",
                    "HEAD",
                ],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toMatch(
                /^https:\/\/localhost:3001\/user\/repo\/commit\/[0-9a-f]{40}$/,
            );
        });

        it("Should generate commit URL with -c flag and HEAD^", () => {
            execSync("git commit -m 'hello world' --allow-empty", {
                cwd: tempDir,
            });

            const result = runGitForge({
                args: [
                    "browse",
                    "--no-browser",
                    "--api",
                    "github",
                    "-c",
                    "HEAD^",
                ],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toMatch(
                /^https:\/\/localhost:3001\/user\/repo\/commit\/[0-9a-f]{40}$/,
            );
        });

        it("Should generate file path URL", () => {
            execSync("mkdir -p src && echo 'fn main() {}' > src/main.rs", {
                cwd: tempDir,
            });

            const result = runGitForge({
                args: [
                    "browse",
                    "--no-browser",
                    "--api",
                    "github",
                    "src/main.rs",
                ],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBe(
                "https://localhost:3001/user/repo/blob/HEAD/src/main.rs",
            );
        });

        it("Should generate file path URL with line number", () => {
            execSync("mkdir -p src && echo 'fn main() {}' > src/main.rs", {
                cwd: tempDir,
            });

            const result = runGitForge({
                args: [
                    "browse",
                    "--no-browser",
                    "--api",
                    "github",
                    "src/main.rs:42",
                ],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBe(
                "https://localhost:3001/user/repo/blob/HEAD/src/main.rs#L42",
            );
        });

        it("Should generate file path URL with commit reference", () => {
            execSync("mkdir -p src && echo 'fn main() {}' > src/main.rs", {
                cwd: tempDir,
            });

            const result = runGitForge({
                args: [
                    "browse",
                    "--no-browser",
                    "--api",
                    "github",
                    "src/main.rs",
                    "-c",
                    "main",
                ],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            // The commit will be resolved to a SHA
            expect(result.stdout).toMatch(
                /^https:\/\/localhost:3001\/user\/repo\/blob\/[0-9a-f]{40}\/src\/main\.rs$/,
            );
        });
    });

    describe("GitLab", () => {
        beforeEach(() => {
            tempDir = setupGitRepo({ forge: "gitlab" });
        });

        it("Should generate repository home URL", () => {
            const result = runGitForge({
                args: ["browse", "--no-browser", "--api", "gitlab"],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBe("https://localhost:3002/user/repo");
        });

        it("Should generate issues list", () => {
            const result = runGitForge({
                args: ["browse", "--no-browser", "--api", "gitlab", "-i"],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBe(
                "https://localhost:3002/user/repo/-/issues",
            );
        });

        it("Should generate specific issue URL", () => {
            const result = runGitForge({
                args: [
                    "browse",
                    "--no-browser",
                    "--api",
                    "gitlab",
                    "-i",
                    "456",
                ],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBe(
                "https://localhost:3002/user/repo/-/issues/456",
            );
        });

        it("Should generate merge requests list URL", () => {
            const result = runGitForge({
                args: ["browse", "--no-browser", "--api", "gitlab", "-p"],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBe(
                "https://localhost:3002/user/repo/-/merge_requests",
            );
        });

        it("Should generate specific merge request URL", () => {
            const result = runGitForge({
                args: [
                    "browse",
                    "--no-browser",
                    "--api",
                    "gitlab",
                    "-p",
                    "789",
                ],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBe(
                "https://localhost:3002/user/repo/-/merge_requests/789",
            );
        });

        it("Should generate commit URL", () => {
            const result = runGitForge({
                args: [
                    "browse",
                    "--no-browser",
                    "--api",
                    "gitlab",
                    "-c",
                    "HEAD",
                ],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toMatch(
                /^https:\/\/localhost:3002\/user\/repo\/-\/commit\/[0-9a-f]{40}$/,
            );
        });

        it("Should generate file path URL", () => {
            execSync("mkdir -p src && echo 'pub fn test() {}' > src/lib.rs", {
                cwd: tempDir,
            });

            const result = runGitForge({
                args: [
                    "browse",
                    "--no-browser",
                    "--api",
                    "gitlab",
                    "src/lib.rs",
                ],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBe(
                "https://localhost:3002/user/repo/-/blob/HEAD/src/lib.rs",
            );
        });

        it("Should generate file path URL with line number", () => {
            execSync("mkdir -p src && echo 'pub fn test() {}' > src/lib.rs", {
                cwd: tempDir,
            });

            const result = runGitForge({
                args: [
                    "browse",
                    "--no-browser",
                    "--api",
                    "gitlab",
                    "src/lib.rs:25",
                ],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBe(
                "https://localhost:3002/user/repo/-/blob/HEAD/src/lib.rs#L25",
            );
        });

        it("Should generate file path URL with commit reference", () => {
            execSync("echo '# README' > README.md", {
                cwd: tempDir,
            });

            const result = runGitForge({
                args: [
                    "browse",
                    "--no-browser",
                    "--api",
                    "gitlab",
                    "README.md",
                    "-c",
                    "main",
                ],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toMatch(
                /^https:\/\/localhost:3002\/user\/repo\/-\/blob\/[0-9a-f]{40}\/README\.md$/,
            );
        });
    });

    describe("Gitea", () => {
        beforeEach(() => {
            tempDir = setupGitRepo({ forge: "gitea" });
        });

        it("Should generate repository home URL", () => {
            const result = runGitForge({
                args: ["browse", "--no-browser", "--api", "gitea"],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBe("https://localhost:3003/user/repo");
        });

        it("Should generate issues list", () => {
            const result = runGitForge({
                args: ["browse", "--no-browser", "--api", "gitea", "-i"],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBe(
                "https://localhost:3003/user/repo/issues",
            );
        });

        it("Should generate specific issue URL", () => {
            const result = runGitForge({
                args: ["browse", "--no-browser", "--api", "gitea", "-i", "999"],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBe(
                "https://localhost:3003/user/repo/issues/999",
            );
        });

        it("Should generate pull requests list URL", () => {
            const result = runGitForge({
                args: ["browse", "--no-browser", "--api", "gitea", "-p"],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBe(
                "https://localhost:3003/user/repo/pulls",
            );
        });

        it("Should generate specific pull request URL", () => {
            const result = runGitForge({
                args: ["browse", "--no-browser", "--api", "gitea", "-p", "111"],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBe(
                "https://localhost:3003/user/repo/pulls/111",
            );
        });

        it("Should generate commit URL", () => {
            const result = runGitForge({
                args: [
                    "browse",
                    "--no-browser",
                    "--api",
                    "gitea",
                    "-c",
                    "HEAD",
                ],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toMatch(
                /^https:\/\/localhost:3003\/user\/repo\/commit\/[0-9a-f]{40}$/,
            );
        });

        it("Should generate file path URL", () => {
            execSync("echo '[package]' > Cargo.toml", {
                cwd: tempDir,
            });

            const result = runGitForge({
                args: [
                    "browse",
                    "--no-browser",
                    "--api",
                    "gitea",
                    "Cargo.toml",
                ],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBe(
                "https://localhost:3003/user/repo/src/commit/HEAD/Cargo.toml",
            );
        });

        it("Should generate file path URL with line number", () => {
            execSync("echo '[package]' > Cargo.toml", {
                cwd: tempDir,
            });

            const result = runGitForge({
                args: [
                    "browse",
                    "--no-browser",
                    "--api",
                    "gitea",
                    "Cargo.toml:10",
                ],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toBe(
                "https://localhost:3003/user/repo/src/commit/HEAD/Cargo.toml#L10",
            );
        });

        it("Should generate file path URL with commit reference", () => {
            execSync("mkdir -p src && echo 'mod cli;' > src/cli.rs", {
                cwd: tempDir,
            });

            const result = runGitForge({
                args: [
                    "browse",
                    "--no-browser",
                    "--api",
                    "gitea",
                    "src/cli.rs",
                    "-c",
                    "main",
                ],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expect(result.stdout).toMatch(
                /^https:\/\/localhost:3003\/user\/repo\/src\/commit\/[0-9a-f]{40}\/src\/cli\.rs$/,
            );
        });
    });
});
