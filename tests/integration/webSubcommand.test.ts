import { execSync } from "node:child_process";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { runCli } from "@tests/integration/utils/cliRunner.js";
import {
    cleanupGitRepo,
    setupGitRepo,
} from "@tests/integration/utils/gitRepo.js";

describe("Web subcommand", () => {
    let tempDir: string;

    afterEach(() => {
        if (tempDir) {
            cleanupGitRepo(tempDir);
            tempDir = "";
        }
    });

    describe("GitHub", () => {
        it("should get web URL for GitHub HTTPS remote", async () => {
            tempDir = setupGitRepo({
                remoteUrl: "https://github.com/user/repo.git",
            });
            const { exitCode, stdout } = await runCli(["web"], {
                cwd: tempDir,
            });

            expect(exitCode).toBe(0);
            expect(stdout.trim()).toBe("https://github.com/user/repo");
        });

        it("should get web URL for GitHub SSH remote", async () => {
            tempDir = setupGitRepo({
                remoteUrl: "ssh://git@github.com:user/repo.git",
            });

            const { exitCode, stdout } = await runCli(["web"], {
                cwd: tempDir,
            });

            expect(exitCode).toBe(0);
            expect(stdout.trim()).toBe("https://github.com/user/repo");
        });

        it("should get web URL for GitHub SSH remote without ssh://", async () => {
            tempDir = setupGitRepo({
                remoteUrl: "git@github.com:user/repo.git",
            });

            const { exitCode, stdout } = await runCli(["web"], {
                cwd: tempDir,
            });

            expect(exitCode).toBe(0);
            expect(stdout.trim()).toBe("https://github.com/user/repo");
        });

        it("should get issues URL with --type issues", async () => {
            tempDir = setupGitRepo({
                remoteUrl: "https://github.com/user/repo.git",
            });

            const { exitCode, stdout } = await runCli(
                ["web", "--type", "issues"],
                { cwd: tempDir },
            );

            expect(exitCode).toBe(0);
            expect(stdout.trim()).toBe("https://github.com/user/repo/issues");
        });

        it("should get pulls URL with --type prs", async () => {
            tempDir = setupGitRepo({
                remoteUrl: "https://github.com/user/repo.git",
            });

            const { exitCode, stdout } = await runCli(
                ["web", "--type", "prs"],
                { cwd: tempDir },
            );

            expect(exitCode).toBe(0);
            expect(stdout.trim()).toBe("https://github.com/user/repo/pulls");
        });

        it("should get pulls URL with --type mrs", async () => {
            tempDir = setupGitRepo({
                remoteUrl: "https://github.com/user/repo.git",
            });

            const { exitCode, stdout } = await runCli(
                ["web", "--type", "mrs"],
                { cwd: tempDir },
            );

            expect(exitCode).toBe(0);
            expect(stdout.trim()).toBe("https://github.com/user/repo/pulls");
        });

        it("should get issues URL with -t short flag", async () => {
            tempDir = setupGitRepo({
                remoteUrl: "https://github.com/user/repo.git",
            });

            const { exitCode, stdout } = await runCli(["web", "-t", "issues"], {
                cwd: tempDir,
            });

            expect(exitCode).toBe(0);
            expect(stdout.trim()).toBe("https://github.com/user/repo/issues");
        });
    });

    describe("GitLab", () => {
        beforeEach(() => {
            tempDir = setupGitRepo({
                remoteUrl: "https://gitlab.com/user/repo.git",
            });
        });

        it("should get web URL for GitLab HTTPS remote", async () => {
            const { exitCode, stdout } = await runCli(["web"], {
                cwd: tempDir,
            });

            expect(exitCode).toBe(0);
            expect(stdout.trim()).toBe("https://gitlab.com/user/repo");
        });

        it("should get web URL for GitLab SSH remote", async () => {
            tempDir = setupGitRepo({
                remoteUrl: "ssh://git@gitlab.com:user/repo.git",
            });

            const { exitCode, stdout } = await runCli(["web"], {
                cwd: tempDir,
            });

            expect(exitCode).toBe(0);
            expect(stdout.trim()).toBe("https://gitlab.com/user/repo");
        });

        it("should get issues URL with --type issues", async () => {
            const { exitCode, stdout } = await runCli(
                ["web", "--type", "issues"],
                { cwd: tempDir },
            );

            expect(exitCode).toBe(0);
            expect(stdout.trim()).toBe("https://gitlab.com/user/repo/-/issues");
        });

        it("should get merge requests URL with --type mrs", async () => {
            const { exitCode, stdout } = await runCli(
                ["web", "--type", "mrs"],
                { cwd: tempDir },
            );

            expect(exitCode).toBe(0);
            expect(stdout.trim()).toBe(
                "https://gitlab.com/user/repo/-/merge_requests",
            );
        });

        it("should get merge requests URL with --type prs", async () => {
            const { exitCode, stdout } = await runCli(
                ["web", "--type", "prs"],
                { cwd: tempDir },
            );

            expect(exitCode).toBe(0);
            expect(stdout.trim()).toBe(
                "https://gitlab.com/user/repo/-/merge_requests",
            );
        });
    });

    describe("Gitea/Forgejo", () => {
        it("should get web URL for Forgejo HTTPS remote", async () => {
            tempDir = setupGitRepo({
                remoteUrl: "https://codeberg.org/user/repo.git",
            });

            const { exitCode, stdout } = await runCli(
                ["web", "--forge-type", "forgejo"],
                { cwd: tempDir },
            );

            expect(exitCode).toBe(0);
            expect(stdout.trim()).toBe("https://codeberg.org/user/repo");
        });

        it("should get web URL for Forgejo SSH remote", async () => {
            tempDir = setupGitRepo({
                remoteUrl: "ssh://git@codeberg.org:user/repo.git",
            });

            const { exitCode, stdout } = await runCli(["web"], {
                cwd: tempDir,
            });

            expect(exitCode).toBe(0);
            expect(stdout.trim()).toBe("https://codeberg.org/user/repo");
        });

        it("should get issues URL with --type issues", async () => {
            tempDir = setupGitRepo({
                remoteUrl: "https://codeberg.org/user/repo.git",
            });

            const { exitCode, stdout } = await runCli(
                ["web", "--forge-type", "forgejo", "--type", "issues"],
                { cwd: tempDir },
            );

            expect(exitCode).toBe(0);
            expect(stdout.trim()).toBe("https://codeberg.org/user/repo/issues");
        });

        it("should get pulls URL with --type prs", async () => {
            tempDir = setupGitRepo({
                remoteUrl: "https://codeberg.org/user/repo.git",
            });

            const { exitCode, stdout } = await runCli(
                ["web", "--forge-type", "forgejo", "--type", "prs"],
                { cwd: tempDir },
            );

            expect(exitCode).toBe(0);
            expect(stdout.trim()).toBe("https://codeberg.org/user/repo/pulls");
        });

        it("should get pulls URL with --type mrs", async () => {
            tempDir = setupGitRepo({
                remoteUrl: "https://codeberg.org/user/repo.git",
            });

            const { exitCode, stdout } = await runCli(
                ["web", "--forge-type", "forgejo", "--type", "mrs"],
                { cwd: tempDir },
            );

            expect(exitCode).toBe(0);
            expect(stdout.trim()).toBe("https://codeberg.org/user/repo/pulls");
        });
    });

    describe("Remote selection", () => {
        it("should use --remote to select non-origin remote", async () => {
            tempDir = setupGitRepo({
                remoteUrl: "https://github.com/user/repo.git",
            });

            execSync(
                `git remote add upstream https://gitlab.com/user/repo.git`,
                { cwd: tempDir },
            );

            const { exitCode, stdout } = await runCli(
                ["web", "--remote", "upstream"],
                { cwd: tempDir },
            );

            expect(exitCode).toBe(0);
            expect(stdout.trim()).toBe("https://gitlab.com/user/repo");
        });
    });

    describe("Help", () => {
        beforeEach(() => {
            tempDir = setupGitRepo({
                remoteUrl: "https://github.com/user/repo.git",
            });
        });

        it("should display help with --help flag", async () => {
            const { exitCode } = await runCli(["web", "--help"], {
                cwd: tempDir,
            });

            expect(exitCode).toBe(0);
        });

        it("should display help with -h flag", async () => {
            const { exitCode } = await runCli(["web", "-h"], {
                cwd: tempDir,
            });

            expect(exitCode).toBe(0);
        });
    });

    describe("Alias", () => {
        it("should work with 'w' alias", async () => {
            tempDir = setupGitRepo({
                remoteUrl: "https://github.com/user/repo.git",
            });

            const { exitCode } = await runCli(["w"], { cwd: tempDir });

            expect(exitCode).toBe(0);
        });
    });

    describe("Errors", () => {
        it("should error when no remote is configured", async () => {
            tempDir = setupGitRepo({
                remoteUrl: "https://github.com/user/repo.git",
            });

            execSync("git remote remove origin", { cwd: tempDir });

            const { exitCode } = await runCli(["web"], { cwd: tempDir });

            expect(exitCode).not.toBe(0);
        });

        it("should error on unsupported forge URL without explicit forge type", async () => {
            tempDir = setupGitRepo({
                remoteUrl: "https://unknown-forge.example.com/user/repo.git",
            });

            const { exitCode } = await runCli(["web"], { cwd: tempDir });

            expect(exitCode).not.toBe(0);
        });

        it("should error on invalid --type value", async () => {
            tempDir = setupGitRepo({
                remoteUrl: "https://github.com/user/repo.git",
            });

            const { exitCode } = await runCli(["web", "--type", "invalid"], {
                cwd: tempDir,
            });

            expect(exitCode).not.toBe(0);
        });
    });
});
