import { CliError, exitWith } from "./debug.js";

export interface HttpClientConfig {
    forgeName: string;
    headers?: Record<string, string>;
}

export class ForgeHttpClient {
    private readonly _forgeName: string;
    private readonly _baseHeaders: Record<string, string>;

    constructor(config: HttpClientConfig) {
        this._forgeName = config.forgeName;
        this._baseHeaders = {
            "User-Agent": "git-forge",
            ...config.headers,
        };
    }

    /**
     * Makes a GET request.
     *
     * @param url - Full API URL to request
     * @returns Promise resolving to parsed JSON response
     */
    async getJson<T>(url: string): Promise<T> {
        const response = await this._executeFetch(
            url,
            "GET",
            this._baseHeaders,
        );

        return await this._handleJsonResponse<T>(response);
    }

    /**
     * Makes a POST request.
     *
     * @param url - Full API URL to request
     * @param body - Request body
     * @returns Promise resolving to parsed JSON response
     */
    async postJson<T>(url: string, body: object): Promise<T> {
        const headers = {
            ...this._baseHeaders,
            "Content-Type": "application/json",
        };
        const response = await this._executeFetch(
            url,
            "POST",
            headers,
            JSON.stringify(body),
        );

        return await this._handleJsonResponse<T>(response);
    }

    private async _executeFetch(
        url: string,
        method: string,
        headers: Record<string, string>,
        body?: string,
    ): Promise<Response> {
        try {
            const sanitizedHeaders = { ...headers };

            if (sanitizedHeaders["Authorization"]) {
                sanitizedHeaders["Authorization"] = "[REDACTED]";
            }

            const response = await fetch(url, {
                method,
                headers,
                body,
            });

            return response;
        } catch (error) {
            exitWith(
                new ForgeError({
                    message: `Failed to fetch from ${url}`,
                    forgeName: this._forgeName,
                    cause: error,
                }),
            );
        }
    }

    private async _handleJsonResponse<T>(response: Response): Promise<T> {
        if (response.status === 401) {
            exitWith(
                new AuthenticationError({
                    message: `Authentication failed. You may need to use the --auth flag and set the appropriate token.`,
                    forgeName: this._forgeName,
                }),
            );
        } else if (!response.ok) {
            exitWith(
                new ForgeError({
                    message: `HTTP ${response.status}: ${await response.text()}`,
                    forgeName: this._forgeName,
                }),
            );
        }

        try {
            return await response.json();
        } catch (error) {
            exitWith(
                new ForgeError({
                    message: `Failed to parse JSON response from ${this._forgeName} API`,
                    forgeName: this._forgeName,
                    cause: error,
                }),
            );
        }
    }
}

export class AuthenticationError extends CliError {
    readonly exitCode = 1;
    readonly userHint = `Please ensure you have set the appropriate token:
  GitHub: GITHUB_TOKEN environment variable
  GitLab: GITLAB_TOKEN environment variable
  Gitea/Forgejo: GITEA_TOKEN environment variable`;
    readonly forgeName: string;
    readonly cause?: unknown;

    constructor({
        message,
        forgeName,
        cause,
    }: {
        message: string;
        forgeName: string;
        cause?: unknown;
    }) {
        super(`[${forgeName}] ${message}`);

        this.name = "AuthenticationError";
        this.forgeName = forgeName;
        this.cause = cause;
    }

    getDebugInfo(): Record<string, unknown> {
        const info: Record<string, unknown> = { forgeName: this.forgeName };

        if (this.cause) {
            info.cause = this.cause;
        }

        return info;
    }
}

export class ForgeError extends CliError {
    readonly exitCode = 1;
    readonly userHint?: string;
    readonly forgeName: string;
    readonly cause?: unknown;

    constructor({
        message,
        forgeName,
        cause,
    }: {
        message: string;
        forgeName: string;
        cause?: unknown;
    }) {
        super(`[${forgeName}] ${message}`);

        this.name = "ForgeError";
        this.forgeName = forgeName;
        this.cause = cause;
    }

    getDebugInfo(): Record<string, unknown> {
        const info: Record<string, unknown> = { forgeName: this.forgeName };

        if (this.cause) {
            info.cause = this.cause;
        }

        return info;
    }
}
