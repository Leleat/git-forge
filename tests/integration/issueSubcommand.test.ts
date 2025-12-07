import { http, HttpResponse } from "msw";
import { setupServer } from "msw/node";
import { execSync } from "node:child_process";
import {
    afterAll,
    afterEach,
    beforeAll,
    beforeEach,
    describe,
    expect,
    it,
} from "vitest";

import { GitHubIssueResponse } from "@/forge/github.js";
import { buildGitHubIssue } from "@tests/integration/utils/builders.js";
import { runCli } from "@tests/integration/utils/cliRunner.js";
import {
    cleanupGitRepo,
    setupGitRepo,
} from "@tests/integration/utils/gitRepo.js";

describe("Issue subcommand", () => {
    const server = setupServer();
    let tempDir: string;

    beforeAll(() => server.listen({ onUnhandledRequest: "error" }));

    beforeEach(() => {
        tempDir = setupGitRepo({
            remoteUrl: "https://github.com/user/repo.git",
        });
    });

    afterEach(() => {
        server.resetHandlers();

        if (tempDir) {
            cleanupGitRepo(tempDir);
            tempDir = "";
        }
    });

    afterAll(() => server.close());

    describe("Alias", () => {
        beforeEach(() => {
            server.use(
                ...githubIssuesGetHandlers({
                    open: [buildGitHubIssue()],
                    closed: [],
                }),
            );
        });

        it("should work with 'i' alias", async () => {
            const { exitCode } = await runCli(["i"], { cwd: tempDir });

            expect(exitCode).toBe(0);
        });
    });

    describe("Help", () => {
        it("should display help with --help flag", async () => {
            const { exitCode } = await runCli(["issue", "--help"], {
                cwd: tempDir,
            });

            expect(exitCode).toBe(0);
        });

        it("should display help with -h flag", async () => {
            const { exitCode } = await runCli(["issue", "-h"], {
                cwd: tempDir,
            });

            expect(exitCode).toBe(0);
        });
    });

    describe("Listing without flags", () => {
        beforeEach(() => {
            server.use(
                ...githubIssuesGetHandlers({
                    open: [
                        buildGitHubIssue({ number: 1, title: "Open issue A" }),
                        buildGitHubIssue({ number: 2, title: "Open issue B" }),
                    ],
                    closed: [
                        buildGitHubIssue({
                            title: "Closed issue",
                            state: "closed",
                        }),
                    ],
                }),
            );
        });

        it("should list open issues with default options", async () => {
            const { exitCode, stdout } = await runCli(["issue"], {
                cwd: tempDir,
            });

            expect(exitCode).toBe(0);

            const [firstLine, secondLine] = stdout.split("\n");

            expect(firstLine).toContain("Open issue A");
            expect(secondLine).toContain("Open issue B");
            expect(stdout).not.toContain("Closed issue");
        });

        it("should output TSV format with tabs", async () => {
            const { exitCode, stdout } = await runCli(["issue"], {
                cwd: tempDir,
            });

            expect(exitCode).toBe(0);

            const rows = stdout
                .trim()
                .split("\n")
                .filter((line) => line);

            expect(rows.length).toBe(2);
            expect(rows[0]).toContain("1 Open issue A");
            expect(rows[1]).toContain("2 Open issue B");
        });

        it("should handle empty issue list", async () => {
            server.use(
                http.get(
                    "https://api.github.com/repos/user/repo/issues",
                    () => {
                        return HttpResponse.json([]);
                    },
                ),
            );

            const { exitCode, stdout } = await runCli(["issue"], {
                cwd: tempDir,
            });

            expect(exitCode).toBe(0);
            expect(stdout.trim()).toBe("");
        });
    });

    describe("Filtering", () => {
        beforeEach(() => {
            server.use(
                ...githubIssuesGetHandlers({
                    open: [
                        buildGitHubIssue({
                            title: "Open issue by alice",
                            user: { login: "alice" },
                            labels: [{ name: "bug" }],
                        }),
                        buildGitHubIssue({
                            title: "Open issue by bob",
                            user: { login: "bob" },
                            labels: [{ name: "feature" }],
                        }),
                    ],
                    closed: [
                        buildGitHubIssue({
                            title: "Closed issue",
                            state: "closed",
                            user: { login: "eve" },
                            labels: [{ name: "wontfix" }],
                        }),
                    ],
                }),
            );
        });

        it("should filter issues by state=open", async () => {
            const { exitCode, stdout } = await runCli(
                ["issue", "--state", "open"],
                { cwd: tempDir },
            );

            expect(exitCode).toBe(0);
            expect(stdout).toContain("Open issue by alice");
            expect(stdout).toContain("Open issue by bob");
            expect(stdout).not.toContain("Closed issue");
        });

        it("should filter issues by state=closed", async () => {
            const { exitCode, stdout } = await runCli(
                ["issue", "--state", "closed"],
                { cwd: tempDir },
            );

            expect(exitCode).toBe(0);
            expect(stdout).not.toContain("Open issue by alice");
            expect(stdout).not.toContain("Open issue by bob");
            expect(stdout).toContain("Closed issue");
        });

        it("should filter issues by state=all", async () => {
            const { exitCode, stdout } = await runCli(
                ["issue", "--state", "all"],
                { cwd: tempDir },
            );

            expect(exitCode).toBe(0);
            expect(stdout).toContain("Open issue by alice");
            expect(stdout).toContain("Open issue by bob");
            expect(stdout).toContain("Closed issue");
        });

        it("should filter issues by author", async () => {
            const { exitCode, stdout } = await runCli(
                ["issue", "--author", "alice"],
                { cwd: tempDir },
            );

            expect(exitCode).toBe(0);
            expect(stdout).toContain("Open issue by alice");
            expect(stdout).not.toContain("Open issue by bob");
            expect(stdout).not.toContain("Closed issue");
        });

        it("should filter issues by labels", async () => {
            const { exitCode, stdout } = await runCli(
                ["issue", "--labels", "bug"],
                { cwd: tempDir },
            );

            expect(exitCode).toBe(0);
            expect(stdout).toContain("Open issue by alice");
            expect(stdout).not.toContain("Open issue by bob");
            expect(stdout).not.toContain("Closed issue");
        });

        it("should filter issues by multiple labels", async () => {
            const { exitCode, stdout } = await runCli(
                ["issue", "--labels", "bug,feature"],
                { cwd: tempDir },
            );

            expect(exitCode).toBe(0);
            expect(stdout).toContain("Open issue by alice");
            expect(stdout).toContain("Open issue by bob");
            expect(stdout).not.toContain("Closed issue");
        });

        it("should return empty results when filter matches nothing", async () => {
            const { exitCode, stdout } = await runCli(
                ["issue", "--author", "nonexistent-user"],
                { cwd: tempDir },
            );

            expect(exitCode).toBe(0);
            expect(stdout.trim()).toBe("");
        });
    });

    describe("Custom columns", () => {
        beforeEach(() => {
            server.use(
                ...githubIssuesGetHandlers({
                    open: [
                        buildGitHubIssue({ number: 1, title: "First issue" }),
                        buildGitHubIssue({ number: 2, title: "Second issue" }),
                    ],
                    closed: [],
                }),
            );
        });

        it("should accept custom --columns flag", async () => {
            const { exitCode, stdout } = await runCli(
                ["issue", "--columns", "id,title"],
                { cwd: tempDir },
            );

            expect(exitCode).toBe(0);

            const rows = stdout
                .trim()
                .split("\n")
                .map((lines) => lines.split("\t"));

            expect(rows.length).toBe(2);
            expect(rows[0].length).toBe(2);
            expect(rows[0][0]).toBe("1");
            expect(rows[0][1]).toBe("First issue");
            expect(rows[1].length).toBe(2);
            expect(rows[1][0]).toBe("2");
            expect(rows[1][1]).toBe("Second issue");
        });
    });

    describe("Pagination", () => {
        beforeEach(() => {
            const allIssues = Array.from({ length: 6 }).map((_, i) =>
                buildGitHubIssue({
                    number: i + 1,
                    title: `Issue ${i + 1}`,
                }),
            );

            server.use(
                http.get(
                    "https://api.github.com/repos/user/repo/issues",
                    ({ request }) => {
                        const url = new URL(request.url);
                        const page = Number(
                            url.searchParams.get("page") || "1",
                        );
                        const perPage = Number(
                            url.searchParams.get("per_page") || "100",
                        );
                        const start = (page - 1) * perPage;
                        const end = start + perPage;
                        const slice = allIssues.slice(start, end);

                        return HttpResponse.json(slice);
                    },
                ),
            );
        });

        it("should return first page respecting --per-page", async () => {
            const { exitCode, stdout } = await runCli(
                ["issue", "--page", "1", "--per-page", "2"],
                { cwd: tempDir },
            );

            expect(exitCode).toBe(0);

            const lines = stdout.trim().split("\n");

            expect(lines).toHaveLength(2);
            expect(lines[0]).toContain("Issue 1");
            expect(lines[1]).toContain("Issue 2");
        });

        it("should return second page respecting --per-page", async () => {
            const { exitCode, stdout } = await runCli(
                ["issue", "--page", "2", "--per-page", "2"],
                { cwd: tempDir },
            );

            expect(exitCode).toBe(0);

            const lines = stdout.trim().split("\n");

            expect(lines).toHaveLength(2);
            expect(lines[0]).toContain("Issue 3");
            expect(lines[1]).toContain("Issue 4");
        });

        it("should return last partial page", async () => {
            const { exitCode, stdout } = await runCli(
                ["issue", "--page", "3", "--per-page", "2"],
                { cwd: tempDir },
            );

            expect(exitCode).toBe(0);

            const lines = stdout.trim().split("\n");

            expect(lines).toHaveLength(2);
            expect(lines[0]).toContain("Issue 5");
            expect(lines[1]).toContain("Issue 6");
        });

        it("should return empty page beyond range", async () => {
            const { exitCode, stdout } = await runCli(
                ["issue", "--page", "4", "--per-page", "2"],
                { cwd: tempDir },
            );

            expect(exitCode).toBe(0);
            expect(stdout.trim()).toBe("");
        });
    });

    describe("Errors", () => {
        it("should error on 400 errors", async () => {
            server.use(
                http.get(
                    "https://api.github.com/repos/user/repo/issues",
                    () => {
                        return HttpResponse.json(
                            { message: "Bad Request" },
                            { status: 400 },
                        );
                    },
                ),
            );

            const { exitCode } = await runCli(["issue"], { cwd: tempDir });

            expect(exitCode).not.toBe(0);
        });

        it("should error on 500 errors", async () => {
            server.use(
                http.get(
                    "https://api.github.com/repos/user/repo/issues",
                    () => {
                        return HttpResponse.json(
                            { message: "Internal Server Error" },
                            { status: 500 },
                        );
                    },
                ),
            );

            const { exitCode } = await runCli(["issue"], { cwd: tempDir });

            expect(exitCode).not.toBe(0);
        });

        it("should error on network errors", async () => {
            server.use(
                http.get(
                    "https://api.github.com/repos/user/repo/issues",
                    () => {
                        return HttpResponse.error();
                    },
                ),
            );

            const { exitCode } = await runCli(["issue"], { cwd: tempDir });

            expect(exitCode).not.toBe(0);
        });

        it("should error when no remote is configured", async () => {
            execSync("git remote remove origin", { cwd: tempDir });

            const { exitCode } = await runCli(["issue"], { cwd: tempDir });

            expect(exitCode).not.toBe(0);
        });

        it("should error on unsupported forge URL without explicit forge type", async () => {
            execSync(
                `git remote set-url origin "https://unknown-forge.example.com/user/repo.git"`,
                { cwd: tempDir },
            );

            const { exitCode } = await runCli(["issue"], { cwd: tempDir });

            expect(exitCode).not.toBe(0);
        });

        it("should error on empty page number", async () => {
            const { exitCode } = await runCli(["issue", "--page", ""], {
                cwd: tempDir,
            });

            expect(exitCode).not.toBe(0);
        });

        it("should error on negative page number", async () => {
            const { exitCode } = await runCli(["issue", "--page", "-1"], {
                cwd: tempDir,
            });

            expect(exitCode).not.toBe(0);
        });

        it("should error on zero page number", async () => {
            const { exitCode } = await runCli(["issue", "--page", "0"], {
                cwd: tempDir,
            });

            expect(exitCode).not.toBe(0);
        });

        it("should error on non-numeric page values", async () => {
            const { exitCode } = await runCli(["issue", "--page", "abc"], {
                cwd: tempDir,
            });

            expect(exitCode).not.toBe(0);
        });

        it("should error on negative per-page values", async () => {
            const { exitCode } = await runCli(["issue", "--per-page", "-5"], {
                cwd: tempDir,
            });

            expect(exitCode).not.toBe(0);
        });

        it("should error on zero per-page values", async () => {
            const { exitCode } = await runCli(["issue", "--per-page", "0"], {
                cwd: tempDir,
            });

            expect(exitCode).not.toBe(0);
        });

        it("should error on non-numeric per-page values", async () => {
            const { exitCode } = await runCli(["issue", "--per-page", "many"], {
                cwd: tempDir,
            });

            expect(exitCode).not.toBe(0);
        });

        it("should error on columns with invalid names", async () => {
            const { exitCode } = await runCli(
                ["issue", "--columns", "id,invalid_column"],
                { cwd: tempDir },
            );

            expect(exitCode).not.toBe(0);
        });
    });
});

function githubIssuesGetHandlers(data: {
    open: GitHubIssueResponse[];
    closed: GitHubIssueResponse[];
}) {
    return [
        http.get(
            "https://api.github.com/repos/user/repo/issues",
            ({ request }) => {
                const url = new URL(request.url);
                const state = url.searchParams.get("state");
                const author = url.searchParams.get("creator");
                const labels = url.searchParams.get("labels");

                if (state === "closed") {
                    return HttpResponse.json(data.closed);
                }

                if (state === "all") {
                    return HttpResponse.json([...data.open, ...data.closed]);
                }

                let filtered = [...data.open];

                if (author) {
                    filtered = filtered.filter(
                        (issue) => issue.user.login === author,
                    );
                }

                if (labels) {
                    const labelList = labels.split(",");
                    filtered = filtered.filter((issue) =>
                        issue.labels.some((label) =>
                            labelList.includes(label.name),
                        ),
                    );
                }

                return HttpResponse.json(filtered);
            },
        ),
    ];
}
