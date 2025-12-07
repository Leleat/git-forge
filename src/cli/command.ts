import { Cli } from "./cli.js";

export interface BareCommand {
    readonly name: "<bare>";

    run(args: string[], cli: Cli): Promise<void>;
}

export interface Subcommand {
    readonly name: "issue" | "pr" | "web";
    readonly aliases: string[];
    readonly description: string;

    run(args: string[]): Promise<void>;
}

export type Command = BareCommand | Subcommand;
