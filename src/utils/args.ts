import {
    ParseArgsConfig,
    ParseArgsOptionsConfig,
    parseArgs as parseArgsFromNode,
} from "node:util";

import { ForgeType, VALID_FORGES as VALID_FORGE_TYPES } from "@/forge/forge.js";
import { CliError, exitWith } from "./debug.js";

export type ParsedFlags<T extends ParseArgsOptionsConfig> = {
    [K in keyof T]: T[K] extends { type: "string"; multiple: true } ? string[]
    : T[K] extends { type: "string"; default: unknown } ? string
    : T[K] extends { type: "string" } ? string | undefined
    : T[K] extends { type: "boolean"; default: unknown } ? boolean
    : T[K] extends { type: "boolean" } ? boolean | undefined
    : never;
};

export interface ParsedArgs<T extends ParseArgsOptionsConfig> {
    positionals: string[];
    flags: ParsedFlags<T>;
}

/**
 * Parses command-line arguments.
 *
 * @param args - Argument array to parse
 * @param config - Parse configuration
 * @returns Parsed arguments with positionals and type-safe flags
 */
export function parseArgs<T extends ParseArgsOptionsConfig>(
    args: string[],
    config: Omit<ParseArgsConfig, "args"> & { options: T },
): ParsedArgs<T> {
    let parsingResults;

    try {
        parsingResults = parseArgsFromNode({
            ...config,
            args,
        });
    } catch (error) {
        exitWith(
            new ArgumentError({
                message:
                    "Error parsing arguments. Use --help for usage information.",
                cause: error,
            }),
        );
    }

    return {
        flags: parsingResults.values as unknown as ParsedFlags<T>,
        positionals: parsingResults.positionals,
    };
}

/**
 * Parses a string value as an integer.
 *
 * @param value - String to parse
 * @returns Parsed integer
 */
export function parseNumber(value: string): number {
    const num = Number.parseInt(value, 10);

    if (isNaN(num)) {
        exitWith(
            new ArgumentError({
                message: `Expected a number, got: ${value}`,
            }),
        );
    }

    return num;
}

/**
 * Parses a comma-separated flag value into array of strings.
 *
 * @param value - Flag value
 * @returns Array of trimmed, non-empty strings. Empty array for undefined or
 *   boolean values
 */
export function parseArrayFlag(value: string | boolean | undefined): string[] {
    return typeof value === "string" ?
            value
                .split(",")
                .map((v) => v.trim())
                .filter((v) => v.length > 0)
        :   [];
}

/**
 * Parses and validates column names from --columns flag. Splits comma-separated
 * column names and validates each against allowed values.
 *
 * @param value - Comma-separated column names
 * @param allowedColumns - Array of valid column names
 * @returns Array of validated column names
 */
export function parseColumns<T extends string>(
    value: string,
    allowedColumns: readonly T[],
): T[] {
    if (typeof value !== "string") {
        exitWith(
            new ArgumentError({
                message: "Expected a comma-separated list of column names",
                flag: "columns",
            }),
        );
    }

    const columns = value
        .split(",")
        .map((c) => c.trim())
        .filter((c) => c.length > 0) as T[];
    const validatedColumns: T[] = [];

    for (const column of columns) {
        if (!allowedColumns.includes(column)) {
            exitWith(
                new ArgumentError({
                    message: `Invalid column name: ${column}. Allowed: ${allowedColumns.join(", ")}`,
                    flag: "columns",
                }),
            );
        }

        validatedColumns.push(column);
    }

    return validatedColumns;
}

function isValidForgeType(value: string): value is ForgeType {
    return VALID_FORGE_TYPES.includes(value as ForgeType);
}

/**
 * Parses and validates --forge-type flag value.
 *
 * @param value - Forge type string from CLI flag
 * @returns Validated ForgeType, or undefined if not provided
 */
export function parseForgeType(
    value: string | undefined,
): ForgeType | undefined {
    if (value === undefined) {
        return undefined;
    }

    if (!isValidForgeType(value)) {
        exitWith(
            new ArgumentError({
                message: `Invalid forge type: ${value}. Allowed: ${VALID_FORGE_TYPES.join(", ")}`,
                flag: "forge-type",
            }),
        );
    }

    return value;
}

export class ArgumentError extends CliError {
    readonly exitCode = 1;
    readonly userHint?: string;
    readonly flag: string | undefined;
    readonly cause?: unknown;

    constructor({
        message,
        flag,
        cause,
    }: {
        message: string;
        flag?: string;
        cause?: unknown;
    }) {
        const isShort = typeof flag === "string" && flag.length === 1;
        const fullName = isShort ? `-${flag}` : `--${flag}`;

        super(flag ? `Invalid argument ${fullName}: ${message}` : message);

        this.name = "ArgumentError";
        this.flag = flag;
        this.cause = cause;
    }

    getDebugInfo(): Record<string, unknown> {
        const info: Record<string, unknown> =
            this.flag ? { flag: this.flag } : {};

        if (this.cause) {
            info.cause = this.cause;
        }

        return info;
    }
}
