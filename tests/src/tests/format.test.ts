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
    { forge: "github" } as const,
    { forge: "gitea" } as const,
    { forge: "gitlab" } as const,
])("Output Format Tests ($forge)", ({ forge }) => {
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

    describe("Issue List Formats", () => {
        it("Should output issues in TSV format", () => {
            const result = runGitForge({
                args: [
                    "issue",
                    "list",
                    "--api",
                    forge,
                    "--api-url",
                    getApiUrl(forge),
                    "--format",
                    "tsv",
                    "--per-page",
                    "3",
                ],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expectTsvFormat(result.stdout);

            const rows = parseTSV(result.stdout);

            expect(rows).toHaveLength(3);
            expect(rows[0]).toHaveProperty("id");
            expect(rows[0]).toHaveProperty("title");
            expect(rows[0]).toHaveProperty("url");
        });

        it("Should output issues in JSON format", () => {
            const result = runGitForge({
                args: [
                    "issue",
                    "list",
                    "--api",
                    forge,
                    "--api-url",
                    getApiUrl(forge),
                    "--format",
                    "json",
                    "--per-page",
                    "3",
                ],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);

            const issues = JSON.parse(result.stdout);

            expect(Array.isArray(issues)).toBeTruthy();
            expect(issues).toHaveLength(3);
            expect(issues[0]).toHaveProperty("id");
            expect(issues[0]).toHaveProperty("title");
            expect(issues[0]).toHaveProperty("url");
        });

        it("Should output issues in JSON format with custom fields", () => {
            const result = runGitForge({
                args: [
                    "issue",
                    "list",
                    "--api",
                    forge,
                    "--api-url",
                    getApiUrl(forge),
                    "--format",
                    "json",
                    "--fields",
                    "id,title",
                    "--per-page",
                    "2",
                ],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);

            const issues = JSON.parse(result.stdout);

            expect(Array.isArray(issues)).toBeTruthy();
            expect(issues).toHaveLength(2);
            expect(issues[0]).toHaveProperty("id");
            expect(issues[0]).toHaveProperty("title");
            expect(issues[0]).not.toHaveProperty("url");
        });
    });

    describe("PR List Formats", () => {
        it("Should output PRs in TSV format", () => {
            const result = runGitForge({
                args: [
                    "pr",
                    "list",
                    "--api",
                    forge,
                    "--api-url",
                    getApiUrl(forge),
                    "--format",
                    "tsv",
                    "--per-page",
                    "3",
                ],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);
            expectTsvFormat(result.stdout);

            const rows = parseTSV(result.stdout);

            expect(rows).toHaveLength(3);
            expect(rows[0]).toHaveProperty("id");
            expect(rows[0]).toHaveProperty("title");
            expect(rows[0]).toHaveProperty("url");
        });

        it("Should output PRs in JSON format", () => {
            const result = runGitForge({
                args: [
                    "pr",
                    "list",
                    "--api",
                    forge,
                    "--api-url",
                    getApiUrl(forge),
                    "--format",
                    "json",
                    "--per-page",
                    "3",
                ],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);

            const prs = JSON.parse(result.stdout);

            expect(Array.isArray(prs)).toBeTruthy();
            expect(prs).toHaveLength(3);
            expect(prs[0]).toHaveProperty("id");
            expect(prs[0]).toHaveProperty("title");
            expect(prs[0]).toHaveProperty("url");
        });

        it("Should output PRs in JSON format with custom fields", () => {
            const result = runGitForge({
                args: [
                    "pr",
                    "list",
                    "--api",
                    forge,
                    "--api-url",
                    getApiUrl(forge),
                    "--format",
                    "json",
                    "--fields",
                    "id,title,draft",
                    "--per-page",
                    "2",
                ],
                cwd: tempDir,
            });

            expect(result.exitCode).toBe(0);

            const prs = JSON.parse(result.stdout);

            expect(Array.isArray(prs)).toBeTruthy();
            expect(prs).toHaveLength(2);
            expect(prs[0]).toHaveProperty("id");
            expect(prs[0]).toHaveProperty("title");
            expect(prs[0]).toHaveProperty("draft");
            expect(prs[0]).not.toHaveProperty("url");
        });
    });
});
