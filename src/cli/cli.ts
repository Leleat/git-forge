import { BareCommand } from "@/cli/bare.js";
import { Command, Subcommand } from "@/cli/command.js";
import { IssueSubcommand } from "@/cli/issue.js";
import { PrSubcommand } from "@/cli/pr.js";
import { WebSubcommand } from "@/cli/web.js";
import { ArgumentError } from "@/utils/args.js";
import { exitWith } from "@/utils/debug.js";

/**
 * Registers commands and executes the CLI.
 *
 * @returns Promise that resolves when command execution completes
 */
export async function main() {
    return new Cli()
        .registerCommand(new BareCommand())
        .registerCommand(new IssueSubcommand())
        .registerCommand(new PrSubcommand())
        .registerCommand(new WebSubcommand())
        .run();
}

export type { Cli };

class Cli {
    private readonly _commands: Command[] = [];
    private readonly _subcommandAliases: Map<string, Subcommand> = new Map();

    /**
     * Returns all registered subcommands.
     *
     * @returns Array of registered subcommands
     */
    getSubcommands(): readonly Subcommand[] {
        return this._commands.filter((c) => c.name !== "<bare>");
    }

    /**
     * Registers a command with the CLI.
     *
     * @param cmd - Command to register
     * @returns The Cli instance for method chaining
     */
    registerCommand(cmd: Command): Cli {
        this._commands.push(cmd);

        if (cmd.name === "<bare>") {
            return this;
        }

        for (const alias of cmd.aliases) {
            this._subcommandAliases.set(alias, cmd);
        }

        return this;
    }

    /**
     * Parses CLI arguments and executes the appropriate (sub)command.
     *
     * @returns Promise that resolves when command execution completes
     */
    async run(): Promise<void> {
        const result = parseCliArgs();
        const commandName = result.command;
        const command =
            this._commands.find((s) => s.name === commandName) ??
            this._subcommandAliases.get(commandName);

        if (!command) {
            const subcommandList = this.getSubcommands()
                .map((s) => s.name)
                .join(", ");

            exitWith(
                `Unknown subcommand: ${commandName}\nAvailable subcommands: ${subcommandList}`,
            );
        }

        return command.run(result.remainingArgs, this);
    }
}

function parseCliArgs(): CliArgsParsingResult {
    const args = process.argv.slice(2);

    if (args.length === 0) {
        exitWith(
            new ArgumentError({
                message: "Expected at least one argument. Use --help for help.",
            }),
        );
    }

    const [maybeSubcommand, ...remaining] = args;

    return maybeSubcommand.startsWith("-") ?
            {
                command: "<bare>",
                remainingArgs: args,
            }
        :   {
                command: maybeSubcommand,
                remainingArgs: remaining,
            };
}

type CliArgsParsingResult =
    | {
          command: "<bare>";
          remainingArgs: string[];
      }
    | {
          command: string;
          remainingArgs: string[];
      };
