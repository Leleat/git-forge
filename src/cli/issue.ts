import { ParseArgsOptionsConfig } from "node:util";

import { Subcommand } from "@/cli/command.js";
import {
    createForge,
    DEFAULT_PAGE,
    DEFAULT_PER_PAGE,
    ForgeArgsOptionsConfig,
    Issue,
    IssueFilters,
} from "@/forge/forge.js";
import {
    ArgumentError,
    parseArgs,
    parseArrayFlag,
    parseColumns,
    ParsedFlags,
    parseForgeType,
    parseNumber,
} from "@/utils/args.js";
import { exitWith } from "@/utils/debug.js";

const ArgsOptionsConfig = {
    ...ForgeArgsOptionsConfig,
    state: {
        type: "string",
        default: "open",
    },
    author: {
        type: "string",
    },
    labels: {
        type: "string",
    },
    page: {
        type: "string",
        default: `${DEFAULT_PAGE}`,
    },
    "per-page": {
        type: "string",
        default: `${DEFAULT_PER_PAGE}`,
    },
    columns: {
        type: "string",
    },
    help: {
        type: "boolean",
        short: "h",
    },
} as const satisfies ParseArgsOptionsConfig;

/**
 * Issue subcommand that lists issues from the repository.
 */
export class IssueSubcommand implements Subcommand {
    readonly name = "issue";
    readonly aliases = ["i"];
    readonly description = "List issues from the remote repository";

    /**
     * Executes the issue list command.
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

        const filters = buildIssueFilters(flags);
        const validCols = [
            "id",
            "title",
            "state",
            "labels",
            "author",
            "created",
            "updated",
            "url",
        ] as const;
        const columns =
            flags.columns ? parseColumns(flags.columns, validCols) : undefined;
        const remote = flags.remote;
        const useAuth = flags.auth;
        const forgeType = parseForgeType(flags["forge-type"]);
        const forge = await createForge(remote, useAuth, forgeType);
        const issues = await forge.listIssues(filters);
        const tsv = formatIssuesToTsv(issues, columns);

        console.log(tsv);
    }
}

function printHelp() {
    console.log(`Usage: git forge issue [<options>]

List issues from the remote repository as TSV

Options:
  --auth                  Use authentication from environment variables
                          (GITHUB_TOKEN, GITLAB_TOKEN, GITEA_TOKEN)
  --author=<user>         Filter by author username
  --columns=<cols>        Columns to include in TSV output (comma-separated)
                          Available: id, title, state, labels, author, created,
                          updated, url
  --forge-type=<type>     Explicitly specify forge type: github, gitlab, gitea,
                          forgejo
  --help, -h              Show this help message
  --labels=<labels>       Filter by labels (comma-separated)
  --page=<number>         Page number to fetch (default: 1)
  --per-page=<number>     Number of issues per page (default: 100)
  --remote=<name>         Git remote to use (default: origin)
  --state=<state>         Filter by state: open, closed, all (default: open)

Output Format (TSV):
  Default: <id> <title>\\t<url>
  Custom:  Columns as specified by --columns

Notes:
  API implementation varies across forges. All filtering options work. Most
  filters are supported server-side. But not all, so we filter client-side for
  those that aren't supported. Client-side filtering means results are filtered
  after fetching from the API, which may affect pagination accuracy.`);
}

function buildIssueFilters(
    flags: ParsedFlags<typeof ArgsOptionsConfig>,
): IssueFilters {
    const filters: IssueFilters = {};
    const state = flags.state;
    const author = flags.author;
    const labelsFlag = flags.labels;
    const pageFlag = flags.page;
    const perPageFlag = flags["per-page"];

    if (state === "open" || state === "closed" || state === "all") {
        filters.state = state;
    }

    if (author) {
        filters.author = author;
    }

    if (labelsFlag) {
        filters.labels = parseArrayFlag(labelsFlag);
    }

    const page = parseNumber(pageFlag);
    const perPage = parseNumber(perPageFlag);

    if (page < 1) {
        exitWith(
            new ArgumentError({
                message: `Page must be >= 1, got: ${page}`,
                flag: "page",
            }),
        );
    }

    if (perPage < 1 || perPage > 100) {
        exitWith(
            new ArgumentError({
                message: `Per-page must be between 1 and 100, got: ${perPage}`,
                flag: "per-page",
            }),
        );
    }

    filters.page = page;
    filters.perPage = perPage;

    return filters;
}

type IssueColumn =
    | "id"
    | "title"
    | "state"
    | "labels"
    | "author"
    | "created"
    | "updated"
    | "url";

function formatIssuesToTsv(issues: Issue[], columns?: IssueColumn[]): string {
    if (!columns) {
        return issues
            .map((i) => `${i.id} ${escapeTsv(i.title)}\t${i.url}`)
            .join("\n");
    }

    return issues
        .map((issue) =>
            columns
                .map((column) => {
                    switch (column) {
                        case "id":
                            return issue.id;
                        case "title":
                            return escapeTsv(issue.title);
                        case "state":
                            return issue.state;
                        case "labels":
                            return escapeTsv(issue.labels.join(","));
                        case "author":
                            return escapeTsv(issue.author);
                        case "created":
                            return issue.createdAt.toISOString();
                        case "updated":
                            return issue.updatedAt.toISOString();
                        case "url":
                            return issue.url;
                    }
                })
                .join("\t"),
        )
        .join("\n");
}

function escapeTsv(value: string | number): string {
    return String(value).replace(/\t/g, " ").replace(/\r?\n/g, " ").trim();
}
