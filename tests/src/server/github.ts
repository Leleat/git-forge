import express, { Request, Response } from "express";

import { GITHUB_PORT } from "../utils.js";
import issues from "./data/github/issue.json";
import prs from "./data/github/pr.json";

interface Label {
    name: string;
}

interface User {
    login: string;
}

interface Issue {
    number: number;
    title: string;
    state: string;
    labels: Label[];
    user: User;
    assignee: User | null;
    html_url: string;
}

interface PullRequest {
    number: number;
    title: string;
    state: string;
    labels: Label[];
    user: User;
    html_url: string;
    created_at: string;
    updated_at: string;
    head: { ref: string };
    base: { ref: string };
    draft: boolean;
    merged_at: string | null;
}

interface CreatePrRequest {
    title: string;
    head: string;
    base: string;
    body?: string;
    draft?: boolean;
}

interface CreateIssueRequest {
    title: string;
    body?: string;
}

export function createGitHubServer(): express.Express {
    const app = express();

    app.use(express.json());

    // GitHub Search API endpoint for issues and PRs
    app.get("/api/v3/search/issues", (req: Request, res: Response) => {
        const { q, page = "1", per_page = "30" } = req.query;

        if (!q || typeof q !== "string") {
            res.status(422).json({ message: "Validation Failed" });
            return;
        }

        // Parse query string
        const query = q.toString();
        const isIssue = query.includes("is:issue");
        const isPR = query.includes("is:pr");
        const isOpen = query.includes("is:open");
        const isClosed = query.includes("is:closed");
        const isUnmerged = query.includes("is:unmerged");
        const isMerged = query.includes("is:merged");
        const isDraft = query.includes("draft:true");

        // Extract search terms (words that are not qualifiers)
        const searchTerms = query
            .split(" ")
            .filter(
                (term) =>
                    !term.startsWith("is:") &&
                    !term.startsWith("repo:") &&
                    !term.startsWith("author:") &&
                    !term.startsWith("assignee:") &&
                    !term.startsWith("label:") &&
                    !term.startsWith("draft:") &&
                    !term.startsWith("in:"),
            )
            .map((term) => term.toLowerCase());

        // Extract qualifiers
        const authorMatch = query.match(/author:(\S+)/);
        const author = authorMatch ? authorMatch[1] : null;

        const assigneeMatch = query.match(/assignee:(\S+)/);
        const assignee = assigneeMatch ? assigneeMatch[1] : null;

        const labelMatches = query.match(/label:(\S+)/g);
        const labels =
            labelMatches ?
                labelMatches.map((l) => l.replace("label:", ""))
            :   [];

        // Start with appropriate items
        let filtered: (Issue | PullRequest)[] = [];
        if (isIssue) {
            filtered = [...issues];
        } else if (isPR) {
            filtered = [...prs];
        }

        // Filter by state
        if (isOpen) {
            filtered = filtered.filter((item) => item.state === "open");
        } else if (isClosed && isUnmerged) {
            // Closed but not merged (PRs only)
            filtered = filtered.filter(
                (item) =>
                    item.state === "closed" &&
                    "merged_at" in item &&
                    !item.merged_at,
            );
        } else if (isMerged) {
            filtered = filtered.filter(
                (item) => "merged_at" in item && item.merged_at !== null,
            );
        } else if (isClosed) {
            filtered = filtered.filter((item) => item.state === "closed");
        }

        // Filter by author
        if (author) {
            filtered = filtered.filter((item) => item.user.login === author);
        }

        // Filter by assignee
        if (assignee) {
            filtered = filtered.filter(
                (item) =>
                    "assignee" in item && item.assignee?.login === assignee,
            );
        }

        // Filter by labels
        if (labels.length > 0) {
            filtered = filtered.filter((item) =>
                labels.every((label) =>
                    item.labels.some((l) => l.name === label),
                ),
            );
        }

        // Filter by draft
        if (isDraft) {
            filtered = filtered.filter(
                (item) => "draft" in item && item.draft === true,
            );
        }

        // Filter by search terms (search in title and body)
        if (searchTerms.length > 0) {
            filtered = filtered.filter((item) =>
                searchTerms.every((term) =>
                    item.title.toLowerCase().includes(term),
                ),
            );
        }

        // Pagination
        const pageNum = Number.parseInt(page as string, 10);
        const perPage = Number.parseInt(per_page as string, 10);
        const start = (pageNum - 1) * perPage;
        const end = start + perPage;
        const paginated = filtered.slice(start, end);

        res.json({ items: paginated });
    });

    // List issues endpoint (kept for backward compatibility)
    app.get(
        "/api/v3/repos/:owner/:repo/issues",
        (req: Request, res: Response) => {
            const {
                state,
                labels,
                creator,
                assignee,
                page = "1",
                per_page = "30",
            } = req.query;
            // Combine issues and PRs (GitHub's /issues endpoint returns both)
            let filtered: (Issue | PullRequest)[] = [...issues, ...prs];

            // Filter by state
            if (state && ["open", "closed"].includes(state.toString())) {
                const s = state.toString();
                filtered = filtered.filter((issue) => issue.state === s);
            }

            // Filter by labels
            if (labels && typeof labels === "string") {
                const requestedLabels = labels.split(",");

                filtered = filtered.filter((issue) =>
                    requestedLabels.every((label) =>
                        issue.labels.some((l) => l.name === label),
                    ),
                );
            }

            // Filter by creator
            if (creator) {
                filtered = filtered.filter(
                    (issue) => issue.user.login === creator,
                );
            }

            // Filter by assignee
            if (assignee) {
                filtered = filtered.filter((issue) => {
                    return (
                        "assignee" in issue &&
                        issue.assignee?.login === assignee
                    );
                });
            }

            // Pagination
            const pageNum = Number.parseInt(page as string, 10);
            const perPage = Number.parseInt(per_page as string, 10);
            const start = (pageNum - 1) * perPage;
            const end = start + perPage;
            const paginated = filtered.slice(start, end);

            res.json(paginated);
        },
    );

    // List pull requests endpoint
    app.get(
        "/api/v3/repos/:owner/:repo/pulls",
        (req: Request, res: Response) => {
            const { state, page = "1", per_page = "30" } = req.query;
            let filtered: PullRequest[] = [...prs];

            // Filter by state
            if (state && ["open", "closed"].includes(state.toString())) {
                const s = state.toString();
                filtered = filtered.filter((pr) => pr.state === s);
            }

            // Pagination
            const pageNum = Number.parseInt(page as string, 10);
            const perPage = Number.parseInt(per_page as string, 10);
            const start = (pageNum - 1) * perPage;
            const end = start + perPage;
            const paginated = filtered.slice(start, end);

            res.json(paginated);
        },
    );

    let prNumber = 1;
    let issueNumber = 1;

    // Create pull request endpoint
    app.post(
        "/api/v3/repos/:owner/:repo/pulls",
        (req: Request, res: Response) => {
            const authHeader = req.headers.authorization;

            if (!authHeader || !authHeader.startsWith("Bearer ")) {
                res.sendStatus(403);

                return;
            }

            const { owner, repo } = req.params;
            const body = req.body as CreatePrRequest;

            if (!body.title || !body.head || !body.base) {
                res.sendStatus(422);

                return;
            }

            const newPr: PullRequest = {
                number: prNumber,
                title: body.title,
                state: "open",
                labels: [],
                user: { login: "test-user" },
                html_url: `http://localhost:${GITHUB_PORT}/${owner}/${repo}/pull/${prNumber}`,
                created_at: new Date().toISOString(),
                updated_at: new Date().toISOString(),
                head: { ref: body.head },
                base: { ref: body.base },
                draft: body.draft || false,
                merged_at: null,
            };

            prNumber++;

            res.status(201).json(newPr);
        },
    );

    // Create issue endpoint
    app.post(
        "/api/v3/repos/:owner/:repo/issues",
        (req: Request, res: Response) => {
            const authHeader = req.headers.authorization;

            if (!authHeader || !authHeader.startsWith("Bearer ")) {
                res.sendStatus(403);

                return;
            }

            const { owner, repo } = req.params;
            const body = req.body as CreateIssueRequest;

            if (!body.title) {
                res.sendStatus(422);

                return;
            }

            const newIssue: Issue = {
                number: issueNumber,
                title: body.title,
                state: "open",
                labels: [],
                user: { login: "test-user" },
                assignee: null,
                html_url: `http://localhost:${GITHUB_PORT}/${owner}/${repo}/issues/${issueNumber}`,
            };

            issueNumber++;

            res.status(201).json(newIssue);
        },
    );

    return app;
}
