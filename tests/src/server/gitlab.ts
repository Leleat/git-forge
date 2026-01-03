import express, { Request, Response } from "express";

import { GITLAB_PORT } from "../utils.js";
import issues from "./data/gitlab/issue.json";
import mrs from "./data/gitlab/mr.json";

interface Author {
    username: string;
}

interface Issue {
    iid: number;
    title: string;
    state: string;
    labels: string[];
    author: Author;
    assignees: Author[];
    web_url: string;
}

interface MergeRequest {
    iid: number;
    title: string;
    state: string;
    labels: string[];
    author: Author;
    web_url: string;
    created_at: string;
    updated_at: string;
    source_branch: string;
    target_branch: string;
    draft: boolean;
}

interface CreateMrRequest {
    source_branch: string;
    target_branch: string;
    title: string;
    description?: string;
}

interface CreateIssueRequest {
    title: string;
    description?: string;
}

export function createGitLabServer(): express.Express {
    const app = express();

    app.use(express.json());

    // List issues endpoint
    app.get(
        "/api/v4/projects/:projectId/issues",
        (req: Request, res: Response) => {
            const {
                state,
                labels,
                assignee_username,
                author_username,
                page = "1",
                per_page = "30",
            } = req.query;
            let filtered: Issue[] = [...issues];

            // Filter by state
            if (state && ["opened", "closed"].includes(state.toString())) {
                const s = state.toString();
                filtered = filtered.filter((issue) => issue.state === s);
            }

            // Filter by labels
            if (labels && typeof labels === "string") {
                const requestedLabels = labels.split(",");

                filtered = filtered.filter((issue) =>
                    requestedLabels.every((label) =>
                        issue.labels.includes(label),
                    ),
                );
            }

            // Filter by assignee
            if (assignee_username) {
                filtered = filtered.filter((issue) =>
                    issue.assignees.some(
                        (a) => a.username === assignee_username,
                    ),
                );
            }

            // Filter by author
            if (author_username) {
                filtered = filtered.filter(
                    (issue) => issue.author.username === author_username,
                );
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

    // List merge requests endpoint
    app.get(
        "/api/v4/projects/:projectId/merge_requests",
        (req: Request, res: Response) => {
            const {
                state,
                labels,
                author_username,
                wip,
                page = "1",
                per_page = "30",
            } = req.query;
            let filtered: MergeRequest[] = [...mrs];

            // Filter by state
            if (
                state &&
                ["opened", "closed", "merged"].includes(state.toString())
            ) {
                const s = state.toString();
                filtered = filtered.filter((mr) => mr.state === s);
            }

            // Filter by labels
            if (labels && typeof labels === "string") {
                const requestedLabels = labels.split(",");

                filtered = filtered.filter((mr) =>
                    requestedLabels.every((label) => mr.labels.includes(label)),
                );
            }

            // Filter by author
            if (author_username) {
                filtered = filtered.filter(
                    (mr) => mr.author.username === author_username,
                );
            }

            // Filter by draft/wip
            if (wip === "yes") {
                filtered = filtered.filter((mr) => mr.draft === true);
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

    let mrIid = 1;
    let issueIid = 1;

    // Create merge request endpoint
    app.post(
        "/api/v4/projects/:projectId/merge_requests",
        (req: Request, res: Response) => {
            const authHeader = req.headers.authorization;

            if (!authHeader || !authHeader.startsWith("Bearer ")) {
                res.sendStatus(401);

                return;
            }

            const body = req.body as CreateMrRequest;

            if (!body.source_branch || !body.target_branch || !body.title) {
                res.sendStatus(422);

                return;
            }

            const newMr: MergeRequest = {
                iid: mrIid,
                title: body.title,
                state: "opened",
                labels: [],
                author: { username: "test-user" },
                web_url: `http://localhost:${GITLAB_PORT}/user/repo/-/merge_requests/${mrIid}`,
                created_at: new Date().toISOString(),
                updated_at: new Date().toISOString(),
                source_branch: body.source_branch,
                target_branch: body.target_branch,
                draft: body.title.startsWith("Draft:"),
            };

            mrIid++;

            res.status(201).json(newMr);
        },
    );

    // Create issue endpoint
    app.post(
        "/api/v4/projects/:projectId/issues",
        (req: Request, res: Response) => {
            const authHeader = req.headers.authorization;

            if (!authHeader || !authHeader.startsWith("Bearer ")) {
                res.sendStatus(401);

                return;
            }

            const body = req.body as CreateIssueRequest;

            if (!body.title) {
                res.sendStatus(422);

                return;
            }

            const newIssue: Issue = {
                iid: issueIid,
                title: body.title,
                state: "opened",
                labels: [],
                author: { username: "test-user" },
                assignees: [],
                web_url: `http://localhost:${GITLAB_PORT}/user/repo/-/issues/${issueIid}`,
            };

            issueIid++;

            res.status(201).json(newIssue);
        },
    );

    return app;
}
