export abstract class CliError extends Error {
    abstract exitCode: number;
    abstract userHint?: string;

    abstract getDebugInfo(): Record<string, unknown>;
}

/**
 * Exits the process with error handling and messaging.
 *
 * @param e - argument for error handling
 */
export function exitWith(e: CliError | Error | string | number): never {
    if (e instanceof CliError) {
        console.error(`${e.message}${e.userHint ? `\n${e.userHint}` : ""}`);

        for (const [key, value] of Object.entries(e.getDebugInfo())) {
            console.error(`${key}: ${value}`);
        }

        return process.exit(e.exitCode);
    } else if (typeof e === "number") {
        process.exit(Number.isNaN(e) ? 1 : e);
    } else {
        console.error(e);
        process.exit(1);
    }
}
