import { ParseArgsOptionsConfig } from "node:util";

import { Subcommand } from "@/cli/command.js";
import {
    createForge,
    DEFAULT_PAGE,
    DEFAULT_PER_PAGE,
    Forge,
    ForgeArgsOptionsConfig,
    Pr,
    PrFilters,
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
import {
    checkoutBranch,
    fetchPullRequest,
    getCurrentBranch,
    getDefaultBranch,
    pushBranch,
} from "@/utils/git.js";

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
    draft: {
        type: "boolean",
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
    title: {
        type: "string",
    },
    body: {
        type: "string",
    },
    target: {
        type: "string",
    },
    push: {
        type: "boolean",
        default: true,
    },
    help: {
        type: "boolean",
        short: "h",
    },
} as const satisfies ParseArgsOptionsConfig;

/**
 * PR subcommand that interacts with pull requests from the repository.
 */
export class PrSubcommand implements Subcommand {
    readonly name = "pr";
    readonly aliases = ["p"];
    readonly description = "Manage pull requests";

    /**
     * Executes the PR subcommand.
     *
     * @param args - Command arguments including action and flags
     * @returns Promise that resolves when command completes
     */
    async run(args: string[]): Promise<void> {
        const { positionals, flags } = parseArgs(args, {
            options: ArgsOptionsConfig,
            allowPositionals: true,
            allowNegative: true,
        });

        if (flags.help) {
            printHelp();

            return;
        }

        const remote = flags.remote;
        const useAuth = flags.auth;
        const forgeType = parseForgeType(flags["forge-type"]);
        const forge = await createForge(remote, useAuth, forgeType);
        const action = positionals[0] || "list";

        switch (action) {
            case "list":
                return runListAction(flags, forge);
            case "open":
                return runOpenAction(flags, forge);
            case "checkout":
                return runCheckoutAction(positionals, forge, remote);
            default:
                exitWith(
                    new ArgumentError({
                        message: `Unknown action: ${action}. Valid actions are: list, open, checkout`,
                        flag: "action",
                    }),
                );
        }
    }
}

async function runListAction(
    flags: ParsedFlags<typeof ArgsOptionsConfig>,
    forge: Forge,
): Promise<void> {
    const filters = buildPrFilters(flags);
    const validCols = [
        "id",
        "title",
        "state",
        "labels",
        "author",
        "created",
        "updated",
        "url",
        "source",
        "target",
        "draft",
    ] as const;
    const columns =
        flags.columns ? parseColumns(flags.columns, validCols) : undefined;
    const prs = await forge.listPrs(filters);
    const tsv = formatPrsToTsv(prs, columns);

    console.log(tsv);
}

async function runOpenAction(
    flags: ParsedFlags<typeof ArgsOptionsConfig>,
    forge: Forge,
): Promise<void> {
    const currentBranch = getCurrentBranch();

    if (!currentBranch) {
        exitWith(
            new ArgumentError({
                message: `Cannot create PR: you are in detached HEAD state. Check out a branch first.`,
            }),
        );
    }

    const targetBranch = flags.target ?? getDefaultBranch(flags.remote);

    if (currentBranch === targetBranch) {
        exitWith(
            new ArgumentError({
                message: `Cannot create PR: current branch "${currentBranch}" is the same as target branch.`,
            }),
        );
    }

    const title = flags.title ?? currentBranch;
    const body = flags.body;
    const draft = flags.draft === true;

    if (flags.push) {
        pushBranch(currentBranch, flags.remote);
    }

    const pr = await forge.createPr({
        title,
        sourceBranch: currentBranch,
        targetBranch,
        body,
        draft,
    });

    console.log(pr.url);
}

async function runCheckoutAction(
    positionals: string[],
    forge: Forge,
    remote: string,
): Promise<void> {
    const prNumber = positionals[1];

    if (!prNumber) {
        exitWith(
            new ArgumentError({
                message:
                    "PR number is required for checkout action. Usage: git forge pr checkout <pr-number>",
                flag: "pr-number",
            }),
        );
    }

    const branchName = `pr-${prNumber}`;
    const ref = forge.getPrRef(prNumber);

    fetchPullRequest(ref, branchName, remote);
    checkoutBranch(branchName);

    console.error(
        `Successfully checked out PR "${prNumber}" to branch "${branchName}"`,
    );
}

function printHelp() {
    console.log(`Usage: git forge pr [<action>] [<options>]

Manage pull requests

Actions:
  list      List pull requests as TSV (default)
  open      Create a new pull request from current branch
  checkout  Checkout a pull request locally

Common options:
  --auth                  Use authentication from environment variables
                          (GITHUB_TOKEN, GITLAB_TOKEN, GITEA_TOKEN)
  --forge-type=<type>     Explicitly specify forge type: github, gitlab, gitea,
                          forgejo
  --help, -h              Show this help message
  --remote=<name>         Git remote to use (default: origin)

Options (for 'list' action):
  --author=<user>         Filter by author username
  --columns=<cols>        Columns to include in TSV output (comma-separated)
                          Available: id, title, state, labels, author, created,
                          updated, url, source, target, draft
  --draft                 Filter to only draft PRs
  --labels=<labels>       Filter by labels (comma-separated)
  --page=<number>         Page number to fetch (default: 1)
  --per-page=<number>     Number of PRs per page (default: 100)
  --state=<state>         Filter by state: open, closed, merged, all
                          (default: open)

Options (for 'open' action):
  --body=<body>           PR description
  --draft                 Create as draft PR
  --push                  Push branch to remote (default: true)
  --target=<branch>       Target branch (defaults to main/master)
  --title=<title>         PR title (defaults to branch name)

Options (for 'checkout' action):
  <pr-number>             PR number to checkout (positional argument)

Output Format (for 'list', TSV):
  Default: <id> <title>\\t<url>
  Custom:  Columns as specified by --columns

Notes:
  API implementation varies across forges. All filtering options work. Most
  filters are supported server-side. But not all, so we filter client-side for
  those that aren't supported. Client-side filtering means results are filtered
  after fetching from the API, which may affect pagination accuracy.`);
}

function buildPrFilters(
    flags: ParsedFlags<typeof ArgsOptionsConfig>,
): PrFilters {
    const filters: PrFilters = {};
    const state = flags.state;
    const author = flags.author;
    const labelsFlag = flags.labels;
    const draft = flags.draft;
    const pageFlag = flags.page;
    const perPageFlag = flags["per-page"];

    if (
        state === "open" ||
        state === "closed" ||
        state === "merged" ||
        state === "all"
    ) {
        filters.state = state;
    }

    if (author) {
        filters.author = author;
    }

    if (labelsFlag) {
        const labels = parseArrayFlag(labelsFlag);

        filters.labels = labels;
    }

    if (typeof draft === "boolean") {
        filters.draft = draft;
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

type PrColumn =
    | "id"
    | "title"
    | "state"
    | "labels"
    | "author"
    | "created"
    | "updated"
    | "url"
    | "source"
    | "target"
    | "draft";

function formatPrsToTsv(prs: Pr[], columns?: PrColumn[]): string {
    if (!columns) {
        return prs
            .map((pr) => `${pr.id} ${escapeTsv(pr.title)}\t${pr.url}`)
            .join("\n");
    }

    return prs
        .map((pr) =>
            columns
                .map((column) => {
                    switch (column) {
                        case "id":
                            return pr.id;
                        case "title":
                            return escapeTsv(pr.title);
                        case "state":
                            return pr.state;
                        case "labels":
                            return escapeTsv(pr.labels.join(","));
                        case "author":
                            return escapeTsv(pr.author);
                        case "created":
                            return pr.createdAt.toISOString();
                        case "updated":
                            return pr.updatedAt.toISOString();
                        case "url":
                            return pr.url;
                        case "source":
                            return pr.sourceBranch;
                        case "target":
                            return pr.targetBranch;
                        case "draft":
                            return pr.draft ? "true" : "false";
                    }
                })
                .join("\t"),
        )
        .join("\n");
}

function escapeTsv(value: string | number): string {
    return String(value).replace(/\t/g, " ").replace(/\r?\n/g, " ").trim();
}
