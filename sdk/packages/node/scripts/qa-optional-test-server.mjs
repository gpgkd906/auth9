#!/usr/bin/env node
import express from "express";
import { auth9Middleware, requirePermission, requireRole } from "@auth9/node/middleware/express";
import { createServer } from "http";

const app = express();
const PORT = 13003;

app.use(auth9Middleware({
  domain: "http://localhost:8080",
  optional: true,
}));

app.get("/public-or-private", (req, res) => {
  if (req.auth) {
    res.json({ message: "Authenticated", user: req.auth.email });
  } else {
    res.json({ message: "Anonymous" });
  }
});

const server = createServer(app);

server.listen(PORT, () => {
  console.log(`Optional mode test server running on http://localhost:${PORT}`);
});

process.on('SIGTERM', () => {
  server.close(() => process.exit(0));
});
