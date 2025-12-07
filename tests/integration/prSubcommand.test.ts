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
    vi,
} from "vitest";

import * as git from "@/utils/git.js";
import { buildGitHubPr } from "@tests/integration/utils/builders.js";
import { runCli } from "@tests/integration/utils/cliRunner.js";
import {
    cleanupGitRepo,
    setupGitRepo,
} from "@tests/integration/utils/gitRepo.js";

describe("PR subcommand", () => {
    const server = setupServer();
    let tempDir: string;

    beforeAll(() => {
        server.listen({ onUnhandledRequest: "error" });

        vi.spyOn(git, "pushBranch").mockImplementation(() => {});
        vi.spyOn(git, "fetchPullRequest").mockImplementation(() => {});
        vi.spyOn(git, "checkoutBranch").mockImplementation(() => {});
    });

    beforeEach(() => {
        tempDir = setupGitRepo({
            remoteUrl: "https://github.com/user/repo.git",
            branch: "feature-branch",
        });
    });

    afterEach(() => {
        server.resetHandlers();
        vi.clearAllMocks();

        if (tempDir) {
            cleanupGitRepo(tempDir);
            tempDir = "";
        }
    });

    afterAll(() => {
        server.close();
        vi.restoreAllMocks();
    });

    describe("Alias", () => {
        beforeEach(() => {
            server.use(
                http.get("https://api.github.com/repos/user/repo/pulls", () => {
                    return HttpResponse.json([buildGitHubPr()]);
                }),
            );
        });

        it("should work with 'p' alias", async () => {
            const { exitCode } = await runCli(["p"], { cwd: tempDir });

            expect(exitCode).toBe(0);
        });
    });

    describe("Help", () => {
        it("should display help with --help flag", async () => {
            const { exitCode } = await runCli(["pr", "--help"], {
                cwd: tempDir,
            });

            expect(exitCode).toBe(0);
        });

        it("should display help with -h flag", async () => {
            const { exitCode } = await runCli(["pr", "-h"], {
                cwd: tempDir,
            });

            expect(exitCode).toBe(0);
        });

        it("should display help for pr list subcommand", async () => {
            const { exitCode } = await runCli(["pr", "list", "--help"], {
                cwd: tempDir,
            });

            expect(exitCode).toBe(0);
        });
    });

    describe("Errors", () => {
        it("should error on 400 errors", async () => {
            server.use(
                http.get("https://api.github.com/repos/user/repo/pulls", () => {
                    return HttpResponse.json(
                        { message: "Bad Request" },
                        { status: 400 },
                    );
                }),
            );

            const { exitCode } = await runCli(["pr"], { cwd: tempDir });

            expect(exitCode).not.toBe(0);
        });

        it("should error on 500 errors", async () => {
            server.use(
                http.get("https://api.github.com/repos/user/repo/pulls", () => {
                    return HttpResponse.json(
                        { message: "Internal Server Error" },
                        { status: 500 },
                    );
                }),
            );

            const { exitCode } = await runCli(["pr"], { cwd: tempDir });

            expect(exitCode).not.toBe(0);
        });

        it("should error on network errors", async () => {
            server.use(
                http.get("https://api.github.com/repos/user/repo/pulls", () => {
                    return HttpResponse.error();
                }),
            );

            const { exitCode } = await runCli(["pr"], { cwd: tempDir });

            expect(exitCode).not.toBe(0);
        });

        it("should error when no remote is configured", async () => {
            execSync("git remote remove origin", { cwd: tempDir });

            const { exitCode } = await runCli(["pr"], { cwd: tempDir });

            expect(exitCode).not.toBe(0);
        });

        it("should error on unsupported forge URL without explicit forge type", async () => {
            execSync(
                `git remote set-url origin "https://unknown-forge.example.com/user/repo.git"`,
                { cwd: tempDir },
            );

            const { exitCode } = await runCli(["pr"], { cwd: tempDir });

            expect(exitCode).not.toBe(0);
        });

        it("should error on empty page number", async () => {
            const { exitCode } = await runCli(["pr", "--page", ""], {
                cwd: tempDir,
            });

            expect(exitCode).not.toBe(0);
        });

        it("should error on negative page number", async () => {
            const { exitCode } = await runCli(["pr", "--page", "-1"], {
                cwd: tempDir,
            });

            expect(exitCode).not.toBe(0);
        });

        it("should error on zero page number", async () => {
            const { exitCode } = await runCli(["pr", "--page", "0"], {
                cwd: tempDir,
            });

            expect(exitCode).not.toBe(0);
        });

        it("should error on non-numeric page values", async () => {
            const { exitCode } = await runCli(["pr", "--page", "abc"], {
                cwd: tempDir,
            });

            expect(exitCode).not.toBe(0);
        });

        it("should error on negative per-page values", async () => {
            const { exitCode } = await runCli(["pr", "--per-page", "-5"], {
                cwd: tempDir,
            });

            expect(exitCode).not.toBe(0);
        });

        it("should error on zero per-page values", async () => {
            const { exitCode } = await runCli(["pr", "--per-page", "0"], {
                cwd: tempDir,
            });

            expect(exitCode).not.toBe(0);
        });

        it("should error on non-numeric per-page values", async () => {
            const { exitCode } = await runCli(["pr", "--per-page", "many"], {
                cwd: tempDir,
            });

            expect(exitCode).not.toBe(0);
        });

        it("should error on columns with invalid names", async () => {
            const { exitCode } = await runCli(
                ["pr", "--columns", "id,invalid_column"],
                { cwd: tempDir },
            );

            expect(exitCode).not.toBe(0);
        });
    });
});
