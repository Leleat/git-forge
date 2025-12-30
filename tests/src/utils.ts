import { execSync, spawnSync } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { expect } from "vitest";

export const GITHUB_PORT = 3001;
export const GITLAB_PORT = 3002;
export const GITEA_PORT = 3003;

const BINARY_PATH = path.resolve(
    import.meta.dirname,
    "../../target/debug/git-forge",
);

export function runGitForge({
    args,
    cwd,
    env = {},
    throwsError = false,
}: {
    args: string[];
    cwd: string;
    env?: Record<string, string>;
    throwsError?: boolean;
}): {
    stdout: string;
    stderr: string;
    exitCode: number;
} {
    const result = spawnSync(BINARY_PATH, args, {
        cwd,
        env,
        encoding: "utf-8",
    });
    const stdout = result.stdout.trim();
    const stderr = result.stderr.trim();
    const exitCode = result.status ?? 0;

    if (exitCode !== 0 && !throwsError) {
        throw new Error(
            `Command failed with exit code ${exitCode}\nstdout: ${stdout}\nstderr: ${stderr}`,
        );
    }

    return {
        stdout,
        stderr,
        exitCode,
    };
}

const TMP_DIR_PREFIX = "git-forge-test-";

export type ApiType = "github" | "gitlab" | "gitea";

export function setupGitRepo({
    forge,
    remoteName = "origin",
}: {
    forge: ApiType;
    remoteName?: string;
}): string {
    const tempDir = mkdtempSync(path.join(tmpdir(), TMP_DIR_PREFIX));

    try {
        execSync("git init --initial-branch=main", { cwd: tempDir });
        execSync('git config user.name "Test User"', { cwd: tempDir });
        execSync('git config user.email "test@example.com"', { cwd: tempDir });
        execSync(`git remote add ${remoteName} ${getRemoteUrl(forge)}`, {
            cwd: tempDir,
        });
        execSync("git commit --allow-empty -m 'Initial commit'", {
            cwd: tempDir,
        });

        return tempDir;
    } catch (error) {
        rmSync(tempDir, { recursive: true, force: true });

        throw error;
    }
}

export function setupGitRepoWithBareRemote({
    forge,
    remoteName = "origin",
}: {
    forge: ApiType;
    remoteName?: string;
}): [string, string] {
    const bareRepoDir = mkdtempSync(path.join(tmpdir(), TMP_DIR_PREFIX));
    const localRepoDir = mkdtempSync(path.join(tmpdir(), TMP_DIR_PREFIX));

    try {
        // Bare Repo
        execSync("git init --bare --initial-branch=main", {
            cwd: bareRepoDir,
            stdio: "ignore",
        });

        // Local Repo
        execSync(`git init --initial-branch=main`, {
            cwd: localRepoDir,
            stdio: "ignore",
        });
        execSync('git config user.name "Test User"', {
            cwd: localRepoDir,
            stdio: "ignore",
        });
        execSync('git config user.email "test@example.com"', {
            cwd: localRepoDir,
            stdio: "ignore",
        });
        execSync(`git remote add ${remoteName} '${getRemoteUrl(forge)}'`, {
            cwd: localRepoDir,
            stdio: "ignore",
        });
        execSync(`git remote set-url --push ${remoteName} '${bareRepoDir}'`, {
            cwd: localRepoDir,
            stdio: "ignore",
        });
        execSync("git commit --allow-empty -m 'Initial commit'", {
            cwd: localRepoDir,
            stdio: "ignore",
        });
        execSync(`git push -u ${remoteName} main`, {
            cwd: localRepoDir,
            stdio: "ignore",
        });
    } catch (error) {
        rmSync(localRepoDir, { recursive: true, force: true });
        rmSync(bareRepoDir, { recursive: true, force: true });

        throw error;
    }

    return [localRepoDir, bareRepoDir];
}

export function cleanupGitRepo(tempDir: string): void {
    const systemTmpDir = tmpdir();

    if (
        !tempDir.startsWith(systemTmpDir) ||
        !tempDir.includes(TMP_DIR_PREFIX)
    ) {
        throw new Error(
            `Refusing to delete "${tempDir}": path must be inside "${systemTmpDir}" and contain "${TMP_DIR_PREFIX}"`,
        );
    }

    try {
        rmSync(tempDir, { recursive: true, force: true });
    } catch (error) {
        console.warn(
            `Warning: Failed to clean up temp directory "${tempDir}":`,
            error,
        );
    }
}

interface TsvRow {
    [column: string]: string;
}

export function parseTSV(
    output: string,
    columns: string[] = ["id", "title", "url"],
): TsvRow[] {
    if (!output.trim()) {
        return [];
    }

    return output.split("\n").map((line) => {
        const values = line.split("\t");
        const row: TsvRow = {};

        columns.forEach((col, index) => {
            row[col] = values[index] || "";
        });

        return row;
    });
}

export function expectTsvFormat(output: string): void {
    expect(output).toBeTruthy();

    for (const line of output.split("\n")) {
        expect(line).toContain("\t");
    }
}

export function getRemoteUrl(forge: ApiType) {
    // The local express server runs on http, but git-forge only accepts https
    // or ssh URLs for parsing
    return {
        github: `https://localhost:${GITHUB_PORT}/user/repo.git`,
        gitlab: `https://localhost:${GITLAB_PORT}/user/repo.git`,
        gitea: `https://localhost:${GITEA_PORT}/user/repo.git`,
    }[forge];
}

export function getApiUrl(forge: ApiType): string {
    return {
        github: `http://localhost:${GITHUB_PORT}/api/v3`,
        gitlab: `http://localhost:${GITLAB_PORT}/api/v4`,
        gitea: `http://localhost:${GITEA_PORT}/api/v1`,
    }[forge];
}
