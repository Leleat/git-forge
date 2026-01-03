import express, { Request, Response } from "express";

import { GITEA_PORT } from "../utils.js";
import issues from "./data/gitea/issue.json";
import prs from "./data/gitea/pr.json";

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
    merged: boolean;
}

interface CreatePrRequest {
    title: string;
    head: string;
    base: string;
    body?: string;
}

interface CreateIssueRequest {
    title: string;
    body?: string;
}

export function createGiteaServer(): express.Express {
    const app = express();

    app.use(express.json());

    // List issues endpoint
    app.get(
        "/api/v1/repos/:owner/:repo/issues",
        (req: Request, res: Response) => {
            const {
                state,
                labels,
                created_by,
                assigned_by,
                page = "1",
                limit = "30",
            } = req.query;
            // Combine issues and PRs (Gitea's /issues endpoint returns both)
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

            // Filter by assignee
            if (assigned_by) {
                filtered = filtered.filter((issue) => {
                    return (
                        "assignee" in issue &&
                        issue.assignee?.login === assigned_by
                    );
                });
            }

            // Filter by creator
            if (created_by) {
                filtered = filtered.filter(
                    (issue) => issue.user.login === created_by,
                );
            }

            // Pagination
            const pageNum = Number.parseInt(page as string, 10);
            const limitNum = Number.parseInt(limit as string, 10);
            const start = (pageNum - 1) * limitNum;
            const end = start + limitNum;
            const paginated = filtered.slice(start, end);

            res.json(paginated);
        },
    );

    // List pull requests endpoint
    app.get(
        "/api/v1/repos/:owner/:repo/pulls",
        (req: Request, res: Response) => {
            const { state, page = "1", limit = "30" } = req.query;
            let filtered: PullRequest[] = [...prs];

            // Filter by state
            if (state && ["open", "closed"].includes(state.toString())) {
                const s = state.toString();
                filtered = filtered.filter((pr) => pr.state === s);
            }

            // Pagination
            const pageNum = Number.parseInt(page as string, 10);
            const limitNum = Number.parseInt(limit as string, 10);
            const start = (pageNum - 1) * limitNum;
            const end = start + limitNum;
            const paginated = filtered.slice(start, end);

            res.json(paginated);
        },
    );

    let prNumber = 0;
    let issueNumber = 0;

    // Create pull request endpoint
    app.post(
        "/api/v1/repos/:owner/:repo/pulls",
        (req: Request, res: Response) => {
            const authHeader = req.headers.authorization;

            if (!authHeader || !authHeader.startsWith("token ")) {
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
                html_url: `http://localhost:${GITEA_PORT}/${owner}/${repo}/pulls/${prNumber}`,
                created_at: new Date().toISOString(),
                updated_at: new Date().toISOString(),
                head: { ref: body.head },
                base: { ref: body.base },
                draft: body.title.startsWith("WIP:"),
                merged: false,
            };

            prNumber++;

            res.status(201).json(newPr);
        },
    );

    // Create issue endpoint
    app.post(
        "/api/v1/repos/:owner/:repo/issues",
        (req: Request, res: Response) => {
            const authHeader = req.headers.authorization;

            if (!authHeader || !authHeader.startsWith("token ")) {
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
                html_url: `http://localhost:${GITEA_PORT}/${owner}/${repo}/issues/${issueNumber}`,
            };

            issueNumber++;

            res.status(201).json(newIssue);
        },
    );

    return app;
}
