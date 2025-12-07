import { execSync } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const PREFIX = "git-forge-test-";

/**
 * Creates a temporary git repository for testing.
 *
 * @param config - Repository configuration
 * @param config.remoteUrl - Remote URL
 * @param config.branch - Current branch name (default: "main")
 * @param config.remoteName - Remote name (default: "origin")
 *
 * @returns Absolute path to the temporary directory
 */
export function setupGitRepo({
    remoteUrl,
    branch = "main",
    remoteName = "origin",
}: {
    remoteUrl: string;
    branch?: string;
    remoteName?: string;
}): string {
    const tempDir = mkdtempSync(join(tmpdir(), PREFIX));

    try {
        execSync("git init --initial-branch=main", { cwd: tempDir });
        execSync('git config user.name "Test User"', { cwd: tempDir });
        execSync('git config user.email "test@example.com"', { cwd: tempDir });
        execSync(`git remote add ${remoteName} ${remoteUrl}`, { cwd: tempDir });
        execSync("git commit --allow-empty -m 'Initial commit'", {
            cwd: tempDir,
        });

        if (branch !== "main") {
            execSync(`git checkout -b ${branch}`, {
                cwd: tempDir,
                stdio: ["ignore"],
            });
        }

        return tempDir;
    } catch (error) {
        rmSync(tempDir, { recursive: true, force: true });

        throw error;
    }
}

/**
 * Removes a temporary git repository created by setupGitRepo.
 *
 * @param tempDir - Path to the temporary directory
 */
export function cleanupGitRepo(tempDir: string): void {
    const systemTmpDir = tmpdir();

    if (!tempDir.startsWith(systemTmpDir) || !tempDir.includes(PREFIX)) {
        throw new Error(
            `Refusing to delete "${tempDir}": path must be inside "${systemTmpDir}" and contain "${PREFIX}"`,
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
