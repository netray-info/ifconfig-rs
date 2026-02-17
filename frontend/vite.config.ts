import { defineConfig, type Plugin } from "vite";
import solidPlugin from "vite-plugin-solid";
import http from "node:http";

const BACKEND = "http://127.0.0.1:8080";
const BACKEND_HOST = "127.0.0.1";
const BACKEND_PORT = 8080;

/**
 * Vite plugin that proxies the root path "/" to the backend when the
 * request looks like an API call (CLI client or Accept: text/plain).
 * Without this, Vite serves its SPA for "/" and the backend's content
 * negotiation never runs.
 */
function rootApiProxy(): Plugin {
  return {
    name: "root-api-proxy",
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        if (req.url !== "/") return next();

        const accept = req.headers.accept || "";
        const ua = req.headers["user-agent"] || "";
        const isCli = /curl|HTTPie|Wget/i.test(ua) && accept === "*/*";
        const wantsPlain = accept.includes("text/plain");

        if (!isCli && !wantsPlain) return next();

        const proxy = http.request(
          {
            hostname: BACKEND_HOST,
            port: BACKEND_PORT,
            path: "/",
            method: req.method,
            headers: req.headers,
          },
          (pRes) => {
            res.writeHead(pRes.statusCode!, pRes.headers);
            pRes.pipe(res);
          },
        );
        proxy.on("error", () => {
          res.writeHead(502);
          res.end("Backend unavailable");
        });
        req.pipe(proxy);
      });
    },
  };
}

export default defineConfig({
  plugins: [rootApiProxy(), solidPlugin()],
  server: {
    proxy: {
      "/ip": BACKEND,
      "/tcp": BACKEND,
      "/host": BACKEND,
      "/location": BACKEND,
      "/isp": BACKEND,
      "/user_agent": BACKEND,
      "/all": BACKEND,
      "/headers": BACKEND,
      "/ipv4": BACKEND,
      "/ipv6": BACKEND,
      "/json": BACKEND,
      "/yaml": BACKEND,
      "/toml": BACKEND,
      "/csv": BACKEND,
      "/health": BACKEND,
      "/meta": BACKEND,
    },
  },
  build: {
    target: "esnext",
  },
});
