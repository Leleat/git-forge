import { execSync } from "node:child_process";
import { Server } from "node:http";

import { createGiteaServer } from "./server/gitea.js";
import { createGitHubServer } from "./server/github.js";
import { createGitLabServer } from "./server/gitlab.js";
import { GITEA_PORT, GITHUB_PORT, GITLAB_PORT } from "./utils.js";

export async function setup() {
    execSync("cargo build", { stdio: "ignore" });

    return startDevServer();
}

export async function teardown() {
    return stopDevServer();
}

let servers: Server[] = [];

async function startDevServer(): Promise<void> {
    if (servers.length > 0) {
        await stopDevServer();
    }

    return new Promise((resolve, reject) => {
        let serversStarted = 0;

        function serverStarted() {
            serversStarted++;

            if (serversStarted === servers.length) {
                resolve();
            }
        }

        function handleError(error: Error) {
            stopDevServer();
            reject(error);
        }

        servers.push(
            createGitHubServer()
                .listen(GITHUB_PORT, () => serverStarted())
                .on("error", handleError),
        );

        servers.push(
            createGitLabServer()
                .listen(GITLAB_PORT, () => serverStarted())
                .on("error", handleError),
        );

        servers.push(
            createGiteaServer()
                .listen(GITEA_PORT, () => serverStarted())
                .on("error", handleError),
        );
    });
}

async function stopDevServer(): Promise<void> {
    if (servers.length === 0) {
        return;
    }

    await Promise.all(
        servers.map(
            (server) =>
                new Promise<void>((resolve) => {
                    server.close(() => resolve());
                }),
        ),
    );

    servers = [];
}
