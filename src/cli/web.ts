import { ParseArgsOptionsConfig } from "node:util";

import { Subcommand } from "@/cli/command.js";
import {
    createForge,
    ForgeArgsOptionsConfig,
    WebUrlType,
} from "@/forge/forge.js";
import { ArgumentError, parseArgs, parseForgeType } from "@/utils/args.js";
import { exitWith } from "@/utils/debug.js";

const ArgsOptionsConfig = {
    ...ForgeArgsOptionsConfig,
    type: {
        type: "string",
        short: "t",
        default: "repository",
    },
    help: {
        type: "boolean",
        short: "h",
    },
} as const satisfies ParseArgsOptionsConfig;

/**
 * Web subcommand that displays repository web URLs.
 */
export class WebSubcommand implements Subcommand {
    readonly name = "web";
    readonly aliases = ["w"];
    readonly description = "Get the web URL for the remote repository";

    /**
     * Executes the web command.
     *
     * @param args - Command arguments
     * @returns Promise that resolves when command completes
     */
    async run(args: string[]): Promise<void> {
        const { flags } = parseArgs(args, { options: ArgsOptionsConfig });

        if (flags.help) {
            printHelp();

            return;
        }

        const remote = flags.remote;
        const forgeType = parseForgeType(flags["forge-type"]);
        const useAuth = flags.auth;
        const webType = parseWebType(flags.type);
        const forge = await createForge(remote, useAuth, forgeType);

        console.log(forge.getWebUrl(webType));
    }
}

function printHelp() {
    console.log(`Usage: git forge web [<options>]

Get the web URL for the remote repository

Options:
  --type, -t <type>  URL type: repository (default), issues, prs (alias: mrs)
  --help, -h         Show this help message

Examples:
  git forge web
  git forge web --type issues
  git forge web -t prs`);
}

function parseWebType(value: string): WebUrlType {
    const VALID_WEB_TYPES = ["repository", "issues", "prs", "mrs"] as const;

    if (!VALID_WEB_TYPES.includes(value as WebUrlType)) {
        exitWith(
            new ArgumentError({
                message: `Invalid type: ${value}. Allowed: ${VALID_WEB_TYPES.join(", ")}`,
                flag: "type",
            }),
        );
    }

    return value as WebUrlType;
}
