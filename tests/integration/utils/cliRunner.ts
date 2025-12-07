import { main } from "@/cli/cli.js";

const FALLBACK_DIR = process.cwd();

/**
 * Runs the git-forge CLI and captures output.
 *
 * @param args - CLI arguments (e.g., ["pr", "list", "--state", "open"])
 * @param options - Execution options
 * @param options.cwd - The current working directory to use
 * @param options.env - The environment variables to use
 * @returns the execution result
 */
export async function runCli(
    args: string[],
    { cwd, env }: { cwd: string; env?: Record<string, string> },
): Promise<{
    stdout: string;
    stderr: string;
    exitCode: number;
}> {
    const originalArgv = process.argv;
    const originalCwd = process.cwd();
    const originalEnv = { ...process.env };
    const originalStdout = process.stdout.write;
    const originalStderr = process.stderr.write;
    const originalConsoleLog = console.log;
    const originalConsoleError = console.error;
    const originalExit = process.exit;

    let stdout = "";
    let stderr = "";
    let exitCode = 0;

    let unhandledRejectionHandler: ((reason: unknown) => void) | undefined;

    try {
        process.argv = ["node", "git-forge", ...args];

        process.chdir(cwd);

        Object.assign(process.env, env);

        process.stdout.write = (chunk: string | Uint8Array) => {
            stdout += chunk.toString();
            return true;
        };

        process.stderr.write = (chunk: string | Uint8Array) => {
            stderr += chunk.toString();
            return true;
        };

        console.log = (...args: unknown[]) => {
            stdout += args.map(String).join(" ") + "\n";
        };

        console.error = (...args: unknown[]) => {
            stderr += args.map(String).join(" ") + "\n";
        };

        process.exit = (code?: number) => {
            throw new ProcessExitError(code ?? 0);
        };

        unhandledRejectionHandler = (reason: unknown) => {
            const errorMsg =
                reason instanceof Error ? reason.stack : String(reason);

            stderr += `Unhandled rejection: ${errorMsg}\n`;
            exitCode = 1;
        };

        process.on("unhandledRejection", unhandledRejectionHandler);

        try {
            await main();
        } catch (error: unknown) {
            if (error instanceof ProcessExitError) {
                exitCode = error.exitCode;
            } else if (error instanceof Error) {
                stderr += error.stack || error.message || String(error);
                stderr += "\n";
                exitCode = 1;
            } else {
                stderr += String(error) + "\n";
                exitCode = 1;
            }
        }
    } finally {
        if (unhandledRejectionHandler) {
            process.off("unhandledRejection", unhandledRejectionHandler);
        }

        process.argv = originalArgv;

        try {
            process.chdir(originalCwd);
        } catch (error) {
            originalConsoleError(
                `Warning: Failed to restore working directory to ${originalCwd}: ${error instanceof Error ? error.message : String(error)}`,
            );

            try {
                process.chdir(FALLBACK_DIR);
            } catch (fallbackError) {
                originalConsoleError(
                    `Critical: Failed to change to fallback directory: ${fallbackError instanceof Error ? fallbackError.message : String(fallbackError)}`,
                );
            }
        }

        process.env = originalEnv;
        process.stdout.write = originalStdout;
        process.stderr.write = originalStderr;
        console.log = originalConsoleLog;
        console.error = originalConsoleError;
        process.exit = originalExit;
    }

    return {
        stdout,
        stderr,
        exitCode,
    };
}

class ProcessExitError extends Error {
    exitCode: number;

    constructor(exitCode: number) {
        super("ProcessExit");

        this.exitCode = exitCode;
    }
}
