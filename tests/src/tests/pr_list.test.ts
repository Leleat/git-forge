import { afterEach, beforeEach, describe, expect, it } from "vitest";

import {
    cleanupGitRepo,
    expectTsvFormat,
    getApiUrl,
    parseTSV,
    runGitForge,
    setupGitRepo,
} from "../utils.js";

describe.each([
    { forge: "github", token: "GITHUB_TOKEN" } as const,
    { forge: "gitea", token: "GITEA_TOKEN" } as const,
    { forge: "gitlab", token: "GITLAB_TOKEN" } as const,
])("PR List Command ($forge)", ({ forge, token }) => {
    let tempDir: string;

    beforeEach(() => {
        tempDir = setupGitRepo({ forge: forge });
    });

    afterEach(() => {
        if (tempDir) {
            cleanupGitRepo(tempDir);
            tempDir = "";
        }
    });

    it("Should display help", () => {
        const result = runGitForge({
            args: ["pr", "list", "--help"],
            cwd: tempDir,
        });

        expect(result.exitCode).toBe(0);
        expect(result.stdout).toBeTruthy();
    });

    it("Should list pull requests using the 'pr list' command", () => {
        const result = runGitForge({
            args: ["pr", "list", "--api", forge, "--api-url", getApiUrl(forge)],
            cwd: tempDir,
        });

        expect(result.exitCode).toBe(0);
        expectTsvFormat(result.stdout);

        const rows = parseTSV(result.stdout);

        expect(rows).toHaveLength(5);
        expect(rows[0]).toHaveProperty("id");
        expect(rows[0]).toHaveProperty("title");
        expect(rows[0]).toHaveProperty("url");
    });

    it("Should list pull requests using the 'p list' alias", () => {
        const result = runGitForge({
            args: ["p", "list", "--api", forge, "--api-url", getApiUrl(forge)],
            cwd: tempDir,
        });

        expect(result.exitCode).toBe(0);
        expectTsvFormat(result.stdout);

        const rows = parseTSV(result.stdout);

        expect(rows).toHaveLength(5);
        expect(rows[0]).toHaveProperty("id");
        expect(rows[0]).toHaveProperty("title");
        expect(rows[0]).toHaveProperty("url");
    });

    it("Should list pull requests using the 'p l' alias", () => {
        const result = runGitForge({
            args: ["p", "l", "--api", forge, "--api-url", getApiUrl(forge)],
            cwd: tempDir,
        });

        expect(result.exitCode).toBe(0);
        expectTsvFormat(result.stdout);

        const rows = parseTSV(result.stdout);

        expect(rows).toHaveLength(5);
        expect(rows[0]).toHaveProperty("id");
        expect(rows[0]).toHaveProperty("title");
        expect(rows[0]).toHaveProperty("url");
    });

    it("Should list pull requests wwhen --auth is used", () => {
        const result = runGitForge({
            args: [
                "pr",
                "list",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--auth",
            ],
            cwd: tempDir,
            env: { [token]: "test-token-123" },
        });

        expect(result.exitCode).toBe(0);
        expectTsvFormat(result.stdout);
    });

    it("Should fail to list pull requests when --auth is used but token isn't set", () => {
        const result = runGitForge({
            args: [
                "pr",
                "list",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--auth",
            ],
            cwd: tempDir,
            throwsError: true,
        });

        expect(result.exitCode).not.toBe(0);
        expect(result.stderr).toBeTruthy();
    });

    it("Should list pull requests with custom columns (id,url)", () => {
        const result = runGitForge({
            args: [
                "pr",
                "list",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--columns",
                "id,url",
            ],
            cwd: tempDir,
        });

        expect(result.exitCode).toBe(0);
        expectTsvFormat(result.stdout);

        const rows = parseTSV(result.stdout, ["id", "url"]);

        expect(rows[0]).toHaveProperty("id");
        expect(rows[0]).toHaveProperty("url");
    });

    it("Should list only draft pull requests", () => {
        const result = runGitForge({
            args: [
                "pr",
                "list",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--draft",
            ],
            cwd: tempDir,
        });

        expect(result.exitCode).toBe(0);
        expectTsvFormat(result.stdout);
        expect(parseTSV(result.stdout)).toHaveLength(2);
    });

    it("Should list pull requests filtered by labels (enhancement+ui)", () => {
        const result = runGitForge({
            args: [
                "pr",
                "list",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--labels",
                "enhancement,ui",
            ],
            cwd: tempDir,
        });

        expect(result.exitCode).toBe(0);
        expect(parseTSV(result.stdout)).toHaveLength(1);
    });

    it("Should list pull requests from page 2", () => {
        const result = runGitForge({
            args: [
                "pr",
                "list",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--page",
                "2",
                "--per-page",
                "2",
            ],
            cwd: tempDir,
        });

        expect(result.exitCode).toBe(0);
        expectTsvFormat(result.stdout);
        expect(parseTSV(result.stdout)).toHaveLength(2);
    });

    it("Should list pull requests with page size of 4", () => {
        const result = runGitForge({
            args: [
                "pr",
                "list",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--per-page",
                "4",
            ],
            cwd: tempDir,
        });

        expect(result.exitCode).toBe(0);
        expectTsvFormat(result.stdout);
        expect(parseTSV(result.stdout)).toHaveLength(4);
    });

    it("Should list pull requests from custom remote (origin)", () => {
        const result = runGitForge({
            args: [
                "pr",
                "list",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--remote",
                "origin",
            ],
            cwd: tempDir,
        });

        expect(result.exitCode).toBe(0);
        expectTsvFormat(result.stdout);
        expect(parseTSV(result.stdout)).toHaveLength(5);
    });

    it("Should fail when given invalid remote name", () => {
        const result = runGitForge({
            args: [
                "pr",
                "list",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--remote",
                "nonexistent",
            ],
            cwd: tempDir,
            throwsError: true,
        });

        expect(result.exitCode).not.toBe(0);
    });

    it("Should list pull requests with state filter (merged)", () => {
        const result = runGitForge({
            args: [
                "pr",
                "list",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--state",
                "merged",
            ],
            cwd: tempDir,
        });

        expect(result.exitCode).toBe(0);
        expectTsvFormat(result.stdout);
        expect(parseTSV(result.stdout)).toHaveLength(4);
    });

    it("Should list pull requests with state filter (closed)", () => {
        const result = runGitForge({
            args: [
                "pr",
                "list",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--state",
                "closed",
            ],
            cwd: tempDir,
        });

        expect(result.exitCode).toBe(0);
        expectTsvFormat(result.stdout);
        expect(parseTSV(result.stdout)).toHaveLength(1);
    });

    it("Should list pull requests filtered by author (bob)", () => {
        const result = runGitForge({
            args: [
                "pr",
                "list",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--author",
                "bob",
            ],
            cwd: tempDir,
        });

        expect(result.exitCode).toBe(0);
        expectTsvFormat(result.stdout);
        expect(parseTSV(result.stdout)).toHaveLength(1);
    });

    it("Should list pull requests with multiple options combined (auth, columns, labels)", () => {
        const result = runGitForge({
            args: [
                "pr",
                "list",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--auth",
                "--columns",
                "id,title,draft",
                "--labels",
                "enhancement",
            ],
            cwd: tempDir,
            env: { [token]: "test-token-123" },
        });

        expect(result.exitCode).toBe(0);
        expectTsvFormat(result.stdout);

        const rows = parseTSV(result.stdout, ["id", "title", "draft"]);

        expect(rows).toHaveLength(3);
        expect(rows[0]).toHaveProperty("id");
        expect(rows[0]).toHaveProperty("title");
        expect(rows[0]).toHaveProperty("draft");
    });
});
