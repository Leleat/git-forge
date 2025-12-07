import { ParseArgsOptionsConfig } from "node:util";

import { Cli } from "@/cli/cli.js";
import { BareCommand as IBareCommand, Subcommand } from "@/cli/command.js";
import { parseArgs } from "@/utils/args.js";
import { version as VERSION } from "../../package.json";

const ArgsOptionsConfig = {
    help: {
        type: "boolean",
        short: "h",
    },
    version: {
        type: "boolean",
        short: "v",
    },
} as const satisfies ParseArgsOptionsConfig;

/**
 * The "Bare" command - i.e. the command that executes when no subcommand is
 * provided (e.g., "git forge --help").
 */
export class BareCommand implements IBareCommand {
    readonly name = "<bare>";

    /**
     * Executes the bare command.
     *
     * @param args - Command arguments
     * @param cli - CLI instance
     * @returns Promise that resolves when command completes
     */
    async run(args: string[], cli: Cli): Promise<void> {
        const { flags } = parseArgs(args, { options: ArgsOptionsConfig });

        if (flags.help) {
            printHelp(cli.getSubcommands());
        } else if (flags.version) {
            console.log(`git-forge version ${VERSION}`);
        }

        process.exit(0);
    }
}

function printHelp(subcommands: readonly Subcommand[]) {
    console.log(`Usage: git forge <subcommand> [<options>]

Tool for basic interactions with git forges

Options:
  --help, -h           Show this help message
  --version, -v        Show version

Subcommands:
${subcommands.map((s) => `  ${buildSubcommandHelpText(s, 21)}`).join("\n")}

Use 'git forge <subcommand> --help' for more information on a specific subcommand.`);
}

function buildSubcommandHelpText(sc: Subcommand, maxPadding: number) {
    const aliasText = sc.aliases.length > 0 ? `, ${sc.aliases.join(", ")}` : "";
    const padding = " ".repeat(
        Math.max(0, maxPadding - sc.name.length - aliasText.length),
    );

    return `${sc.name}${aliasText}${padding}${sc.description}`;
}
