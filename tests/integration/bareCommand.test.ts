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
import { runCli } from "@tests/integration/utils/cliRunner.js";
import {
    cleanupGitRepo,
    setupGitRepo,
} from "@tests/integration/utils/gitRepo.js";

describe("Bare command", () => {
    let tempDir = "";

    beforeAll(() => {
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
        if (tempDir) {
            cleanupGitRepo(tempDir);
            tempDir = "";
        }
    });

    afterAll(() => {
        vi.restoreAllMocks();
    });

    describe("Help", () => {
        it("should display help with --help flag", async () => {
            const { exitCode } = await runCli(["--help"], { cwd: tempDir });

            expect(exitCode).toBe(0);
        });

        it("should display help with -h flag", async () => {
            const { exitCode } = await runCli(["-h"], { cwd: tempDir });

            expect(exitCode).toBe(0);
        });

        it("should fail if no arguments given", async () => {
            const { exitCode } = await runCli([], { cwd: tempDir });

            expect(exitCode).toBe(1);
        });
    });

    describe("Version", () => {
        it("should display version with --version flag", async () => {
            const { exitCode, stdout } = await runCli(["--version"], {
                cwd: tempDir,
            });

            expect(exitCode).toBe(0);
            expect(stdout).toMatch(/\d+\.\d+\.\d+/);
        });

        it("should display version with -v flag", async () => {
            const { exitCode, stdout } = await runCli(["-v"], {
                cwd: tempDir,
            });

            expect(exitCode).toBe(0);
            expect(stdout).toMatch(/\d+\.\d+\.\d+/);
        });
    });

    describe("Unknown subcommands", () => {
        it("should error on unknown subcommand", async () => {
            const { exitCode } = await runCli(["unknown-subcommand"], {
                cwd: tempDir,
            });

            expect(exitCode).not.toBe(0);
        });
    });

    describe("Unknown flags", () => {
        it("should reject unknown flags", async () => {
            const { exitCode } = await runCli(["--unknown-flag"], {
                cwd: tempDir,
            });

            expect(exitCode).not.toBe(0);
        });
    });
});
