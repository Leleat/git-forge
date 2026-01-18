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
    { forge: "github", token: "GIT_FORGE_GITHUB_TOKEN" } as const,
    { forge: "gitea", token: "GIT_FORGE_GITEA_TOKEN" } as const,
    { forge: "gitlab", token: "GIT_FORGE_GITLAB_TOKEN" } as const,
])("Issue List Command ($forge)", ({ forge, token }) => {
    let tempDir: string;

    beforeEach(() => {
        tempDir = setupGitRepo({ forge });
    });

    afterEach(() => {
        if (tempDir) {
            cleanupGitRepo(tempDir);
            tempDir = "";
        }
    });

    it("Should display help", () => {
        const result = runGitForge({
            args: ["issue", "--help"],
            cwd: tempDir,
        });

        expect(result.exitCode).toBe(0);
        expect(result.stdout).toBeTruthy();
    });

    it("Should list issues using full 'issue' command", () => {
        const result = runGitForge({
            args: [
                "issue",
                "list",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
            ],
            cwd: tempDir,
        });

        expect(result.exitCode).toBe(0);
        expectTsvFormat(result.stdout);

        const rows = parseTSV(result.stdout);

        expect(rows).toHaveLength(7);
        expect(rows[0]).toHaveProperty("id");
        expect(rows[0]).toHaveProperty("title");
        expect(rows[0]).toHaveProperty("url");
    });

    it("Should list issues using 'i' alias", () => {
        const result = runGitForge({
            args: ["i", "list", "--api", forge, "--api-url", getApiUrl(forge)],
            cwd: tempDir,
        });

        expect(result.exitCode).toBe(0);
        expectTsvFormat(result.stdout);

        const rows = parseTSV(result.stdout);

        expect(rows).toHaveLength(7);
    });

    it("Should list issues with --auth", () => {
        const result = runGitForge({
            args: [
                "issue",
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

    it("Should fail to list issues when --auth is used but token isn't set", () => {
        const result = runGitForge({
            args: [
                "issue",
                "list",
                "--api",
                forge,
                "--auth",
                "--api-url",
                getApiUrl(forge),
            ],
            cwd: tempDir,
            throwsError: true,
        });

        expect(result.exitCode).not.toBe(0);
        expect(result.stderr).toBeTruthy();
    });

    it("Should list issues with custom fields (id,title)", () => {
        const result = runGitForge({
            args: [
                "issue",
                "list",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--fields",
                "id,title",
            ],
            cwd: tempDir,
        });

        expect(result.exitCode).toBe(0);
        expectTsvFormat(result.stdout);

        const rows = parseTSV(result.stdout, ["id", "title"]);

        expect(rows[0]).toHaveProperty("id");
        expect(rows[0]).toHaveProperty("title");
    });

    it("Should list issues filtered by labels (enhancement+high-priority)", () => {
        const result = runGitForge({
            args: [
                "issue",
                "list",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--labels",
                "enhancement,high-priority",
            ],
            cwd: tempDir,
        });

        expect(result.exitCode).toBe(0);
        expectTsvFormat(result.stdout);
        expect(parseTSV(result.stdout)).toHaveLength(1);
    });

    it("Should list issues from page 2", () => {
        const result = runGitForge({
            args: [
                "issue",
                "list",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--page",
                "2",
                "--per-page",
                "3",
            ],
            cwd: tempDir,
        });

        expect(result.exitCode).toBe(0);
        expectTsvFormat(result.stdout);
        expect(parseTSV(result.stdout)).toHaveLength(3);
    });

    it("Should list issues with page size of 5", () => {
        const result = runGitForge({
            args: [
                "issue",
                "list",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--per-page",
                "5",
            ],
            cwd: tempDir,
        });

        expect(result.exitCode).toBe(0);
        expectTsvFormat(result.stdout);
        expect(parseTSV(result.stdout)).toHaveLength(5);
    });

    it("Should list issues from custom remote (origin)", () => {
        const result = runGitForge({
            args: [
                "issue",
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
        expect(parseTSV(result.stdout)).toHaveLength(7);
    });

    it("Should fail when given invalid remote name", () => {
        const result = runGitForge({
            args: [
                "issue",
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

    it("Should list issues filtered by assignee (bob)", () => {
        const result = runGitForge({
            args: [
                "issue",
                "list",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--assignee",
                "bob",
            ],
            cwd: tempDir,
        });

        expect(result.exitCode).toBe(0);
        expectTsvFormat(result.stdout);
        expect(parseTSV(result.stdout)).toHaveLength(3);
    });

    it("Should list issues filtered by author (alice)", () => {
        const result = runGitForge({
            args: [
                "issue",
                "list",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--author",
                "alice",
            ],
            cwd: tempDir,
        });

        expect(result.exitCode).toBe(0);
        expectTsvFormat(result.stdout);
        expect(parseTSV(result.stdout)).toHaveLength(3);
    });

    it("Should list issues with state filter (all)", () => {
        const result = runGitForge({
            args: [
                "issue",
                "list",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--state",
                "all",
            ],
            cwd: tempDir,
        });

        expect(result.exitCode).toBe(0);
        expectTsvFormat(result.stdout);
        expect(parseTSV(result.stdout)).toHaveLength(10);
    });

    it("Should list issues with state filter (closed)", () => {
        const result = runGitForge({
            args: [
                "issue",
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
        expect(parseTSV(result.stdout)).toHaveLength(3);
    });

    it("Should list issues with multiple options combined (auth, fields, labels)", () => {
        const result = runGitForge({
            args: [
                "issue",
                "list",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--auth",
                "--fields",
                "id,title,author",
                "--labels",
                "enhancement",
            ],
            cwd: tempDir,
            env: { [token]: "test-token-123" },
        });

        expect(result.exitCode).toBe(0);
        parseTSV(result.stdout);

        const rows = parseTSV(result.stdout, ["id", "title", "author"]);

        expect(rows).toHaveLength(4);
        expect(rows[0]).toHaveProperty("id");
        expect(rows[0]).toHaveProperty("title");
        expect(rows[0]).toHaveProperty("author");
    });

    it("Should filter issues by query (Fix)", () => {
        const result = runGitForge({
            args: [
                "issue",
                "list",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--query",
                "Fix",
                "--state",
                "all",
                "--fields",
                "id,title,url",
            ],
            cwd: tempDir,
        });

        expect(result.exitCode).toBe(0);
        expectTsvFormat(result.stdout);

        const rows = parseTSV(result.stdout);

        expect(rows).toHaveLength(2);

        rows.forEach((row) => {
            expect(row.title).toContain("Fix");
        });
    });

    it("Should return no results for query with no matches", () => {
        const result = runGitForge({
            args: [
                "issue",
                "list",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--query",
                "nonexistentterm",
                "--fields",
                "id,title,url",
            ],
            cwd: tempDir,
        });

        expect(result.exitCode).toBe(0);
        expect(parseTSV(result.stdout)).toHaveLength(0);
    });
});
