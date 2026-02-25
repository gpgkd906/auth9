import { PassThrough } from "stream";
import crypto from "crypto";
import type { EntryContext } from "react-router";
import { createReadableStreamFromReadable } from "@react-router/node";
import { ServerRouter } from "react-router";
import { isbot } from "isbot";
import { renderToPipeableStream } from "react-dom/server";
import { NonceContext } from "~/hooks/useNonce";

const ABORT_DELAY = 5_000;

function generateNonce(): string {
  return crypto.randomBytes(16).toString("base64");
}

function setSecurityHeaders(headers: Headers, nonce: string, request?: Request): void {
  // Prevent browser caching of authenticated pages (mitigates bfcache leaking sensitive data)
  headers.set("Cache-Control", "no-store, no-cache, must-revalidate, private");
  headers.set("Pragma", "no-cache");
  // Prevent MIME type sniffing
  headers.set("X-Content-Type-Options", "nosniff");
  // Prevent clickjacking
  headers.set("X-Frame-Options", "DENY");
  // XSS protection (legacy but useful for older browsers)
  headers.set("X-XSS-Protection", "1; mode=block");
  // Control referrer information
  headers.set("Referrer-Policy", "strict-origin-when-cross-origin");
  // Restrict browser features
  headers.set(
    "Permissions-Policy",
    "camera=(), microphone=(), geolocation=(), payment=()"
  );
  // Content Security Policy - nonce-based for React Router 7 hydration scripts
  headers.set(
    "Content-Security-Policy",
    [
      "default-src 'self'",
      `script-src 'self' 'nonce-${nonce}'`,
      "style-src 'self' 'unsafe-inline' https://fonts.googleapis.com",
      "img-src 'self' data: https:",
      "font-src 'self' data: https://fonts.gstatic.com",
      "connect-src 'self' http://localhost:* https://localhost:* ws://localhost:*",
      "frame-ancestors 'none'",
      "base-uri 'self'",
      "form-action 'self'",
    ].join("; ")
  );
  // HTTP Strict Transport Security - only set when served over HTTPS
  const isHttps = request
    ? request.headers.get("X-Forwarded-Proto") === "https" ||
      new URL(request.url).protocol === "https:"
    : false;
  if (isHttps) {
    headers.set("Strict-Transport-Security", "max-age=31536000; includeSubDomains");
  }
}

export default function handleRequest(
  request: Request,
  responseStatusCode: number,
  responseHeaders: Headers,
  routerContext: EntryContext
) {
  const nonce = generateNonce();

  return isbot(request.headers.get("user-agent") || "")
    ? handleBotRequest(
        request,
        responseStatusCode,
        responseHeaders,
        routerContext,
        nonce
      )
    : handleBrowserRequest(
        request,
        responseStatusCode,
        responseHeaders,
        routerContext,
        nonce
      );
}

function handleBotRequest(
  request: Request,
  responseStatusCode: number,
  responseHeaders: Headers,
  routerContext: EntryContext,
  nonce: string
) {
  return new Promise((resolve, reject) => {
    let shellRendered = false;
    const { pipe, abort } = renderToPipeableStream(
      <NonceContext.Provider value={nonce}>
        <ServerRouter context={routerContext} url={request.url} nonce={nonce} />
      </NonceContext.Provider>,
      {
        nonce,
        onAllReady() {
          shellRendered = true;
          const body = new PassThrough();
          const stream = createReadableStreamFromReadable(body);

          responseHeaders.set("Content-Type", "text/html");
          setSecurityHeaders(responseHeaders, nonce, request);

          resolve(
            new Response(stream, {
              headers: responseHeaders,
              status: responseStatusCode,
            })
          );

          pipe(body);
        },
        onShellError(error: unknown) {
          reject(error);
        },
        onError(error: unknown) {
          responseStatusCode = 500;
          if (shellRendered) {
            console.error(error);
          }
        },
      }
    );

    setTimeout(abort, ABORT_DELAY);
  });
}

function handleBrowserRequest(
  request: Request,
  responseStatusCode: number,
  responseHeaders: Headers,
  routerContext: EntryContext,
  nonce: string
) {
  return new Promise((resolve, reject) => {
    let shellRendered = false;
    const { pipe, abort } = renderToPipeableStream(
      <NonceContext.Provider value={nonce}>
        <ServerRouter context={routerContext} url={request.url} nonce={nonce} />
      </NonceContext.Provider>,
      {
        nonce,
        onShellReady() {
          shellRendered = true;
          const body = new PassThrough();
          const stream = createReadableStreamFromReadable(body);

          responseHeaders.set("Content-Type", "text/html");
          setSecurityHeaders(responseHeaders, nonce, request);

          resolve(
            new Response(stream, {
              headers: responseHeaders,
              status: responseStatusCode,
            })
          );

          pipe(body);
        },
        onShellError(error: unknown) {
          reject(error);
        },
        onError(error: unknown) {
          responseStatusCode = 500;
          if (shellRendered) {
            console.error(error);
          }
        },
      }
    );

    setTimeout(abort, ABORT_DELAY);
  });
}
