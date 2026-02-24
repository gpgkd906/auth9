# An Experiment: Can AI-Native Software Development Lifecycle "Polish" Software Like Humans Do?

*Testing Shifted Left — Again*

---

## The Experiment's Purpose

I didn't set out to build an identity platform. I wanted to answer a more fundamental question: **can an AI-native development process actually produce a polished application?**

Not a toy demo, not a weekend project. A real system — complex enough to test the methodology, with security requirements high enough that "good enough" simply doesn't cut it.

I chose IAM (Identity and Access Management) as the test subject. This isn't simple CRUD: multi-tenant data isolation, OIDC/OAuth2 flows, Token Exchange, hierarchical RBAC permissions, webhook signature verification, audit logging — this interconnected complexity means one wrong decision can cascade into a dozen subtle bugs. Here, security isn't a nice-to-have; it's the very reason the system exists.

The result is [Auth9](https://github.com/gpgkd906/auth9), a self-hosted alternative to Auth0 built with Rust, React Router 7, TiDB, and Keycloak. Almost all code in this project was AI-generated, and almost every step was driven by skills — from the backend Rust services to frontend React components, from test cases to deployment scripts. But Auth9 is just the output. The real subject of the experiment is this development pipeline itself.

## The Real Challenge: Verifiability

AI coding tools do make you write code faster. GitHub Copilot, Cursor, Claude Code — they all deliver on that promise.

But writing code was never the real difficulty. The difficulty is **knowing whether the code is correct** — and knowing it fast enough, automated enough, that verification doesn't become the bottleneck.

Can you verify that a refactor didn't break three seemingly unrelated flows? Can you confirm a security fix didn't regress the permission model? Can you keep 156 test documents in sync when a route changes — not next week, but *right now*, within the same development cycle? Humans can certainly do these verifications, but it takes time — and time is precisely what AI-native development aims to solve.

The AI-native development process doesn't eliminate verification work. It makes verification **systematic and automated enough** to keep pace with AI-speed code generation. If AI writes code 10x faster but verification stays manual, you've just created a 10x larger QA backlog.

## Testing Shifted Left

Let me be clear: **testing didn't disappear. It became more important.** What changed is the form.

Traditional automated tests — unit tests, integration tests, end-to-end tests — still exist in the codebase. `cargo test` runs fast with no external dependencies. Playwright drives end-to-end flows. Vitest covers the frontend. These techniques remain effective and essential. Of course, they are all AI-generated.

What we added is a layer *before* code-level tests: **QA test documents**. These are structured specifications that describe what to test, how to test it, and how to verify correctness at the data layer. AI generates them; humans review and approve them. Then AI executes them — including browser automation, API calls, database queries, and gRPC validation. In essence, it's closer to traditional manual testing, but automated. Of course, it can't completely replace manual testing yet.

This is testing shifted left into documentation. The test plan isn't an afterthought scribbled after the feature ships. It's the first artifact produced after a feature is planned, and it drives everything downstream: what code gets written, what gets verified, and what gets caught when something breaks.

The human's role: review every generated test document for completeness, edge cases, and security considerations the AI might miss; observe the agent's automated testing to check if its testing behavior meets expectations, or if there's any cheating. The AI's role: generate the documents, execute them, report failures, and fix what it can. And the QA test documents themselves reflect the user stories.

## The Closed-Loop Pipeline

The pipeline chains 16 Agent Skills together, where the output of each phase feeds the next:

```
Human + AI ──► Plan feature
                  │
                  ▼
          ┌─ Generate QA / Security / UIUX test docs
          │   (qa-doc-gen)
          ▼
          ┌─ Execute tests automatically
          │   Browser automation, API testing,
          │   DB validation, gRPC regression,
          │   performance benchmarks
          ▼
          ┌─ Failures? Create structured tickets
          │   (docs/ticket/)
          ▼
          ┌─ AI reads ticket → verifies issue →
          │   fixes code → resets environment →
          │   re-runs tests → closes ticket
          │   (ticket-fix)
          ▼
          ┌─ Periodically audit doc quality
          │   (qa-doc-governance)
          ▼
          ┌─ Align tests after refactors
          │   (align-tests, test-coverage)
          ▼
          ┌─ Deploy to Kubernetes
          │   (deploy-gh-k8s)
          └─────────────────────────
```

Each skill is a self-contained Agent Skill definition — a markdown file with instructions, workflow steps, and tool scripts:

| Phase | Skills | What They Do |
|-------|--------|-------------|
| **Plan** | `project-bootstrap` | Scaffold a new project from scratch |
| **Code** | `rust-conventions`, `keycloak-theme` | Coding standards, theme development |
| **Test Docs** | `qa-doc-gen`, `qa-doc-governance` | Generate and govern test documentation |
| **Execute Tests** | `qa-testing`, `e2e-testing`, `performance-testing`, `auth9-grpc-regression` | Run QA, E2E, load, and gRPC tests |
| **Fix** | `ticket-fix`, `align-tests` | Auto-fix tickets, realign tests after refactors |
| **Coverage** | `test-coverage` | Enforce >=90% coverage across all layers |
| **Deploy** | `deploy-gh-k8s` | GitHub Actions gate → K8s deploy → health check |
| **Operate** | `ops`, `reset-local-env` | Logs, troubleshooting, environment reset |

The key insight: **Agent Skills compose**. After a feature is planned, `qa-doc-gen` generates test documents. `qa-testing` executes them. Failures become tickets in `docs/ticket/`. `ticket-fix` reads the tickets, fixes the code, resets the environment, re-runs the tests, and closes the tickets. `qa-doc-governance` periodically audits everything to prevent documentation rot.

## Documents as Executable Specs

This only works because test documents are structured for both humans and AI. Here's a real scenario from the tenant CRUD test suite:

> ### Scenario 1: Tenant Management Entry Visibility and Create Tenant
>
> **Initial State**
> - User is logged into the admin dashboard
> - No tenant with the same name or slug exists in the database
>
> **Test Steps**
> 1. Confirm "Tenant Management" menu entry exists in the left sidebar
> 2. Click "Tenant Management" to enter the tenant list
> 3. Click "Create Tenant" button
> 4. Fill in the form:
>    - Name: `Test Company`
>    - Slug: `test-company`
>    - Logo URL: `https://example.com/logo.png`
> 5. Click "Create" button
>
> **Expected Results**
> - Success notification displayed
> - Sidebar entry visible and clickable
> - Tenant appears in list with "Active" status
>
> **Expected Data State**
> ```sql
> SELECT id, name, slug, logo_url, status FROM tenants WHERE slug = 'test-company';
> -- Expected: one record exists, status = 'active'
>
> SELECT action, resource_type FROM audit_logs WHERE resource_type = 'tenant' ORDER BY created_at DESC LIMIT 1;
> -- Expected: action = 'tenant.create'
> ```

The SQL verification is the critical piece. The AI doesn't just check if the UI says "success" — it queries the database to confirm the data actually landed correctly. This catches an entire class of bugs where the frontend shows success but the backend silently failed.

We have **156 documents** like this: 96 QA, 48 security, 12 UI/UX — each one an executable specification, AI-generated and human-reviewed.

We chose the filesystem as the information exchange medium (e.g., `docs/qa`, `docs/ticket` directories) for a simple reason — Agents are excellent at bash operations, and file system interactions are natural, precise, and efficient. This file-based workflow makes the entire verification process transparent and traceable.

## The Self-Healing Loop: How AI "Polishes" Software

The `ticket-fix` skill is the most interesting part of the pipeline, and it's the core mechanism of how AI "polishes" software. When a test fails, a structured ticket is created. Then:

1. **Read ticket and validate** — Parse the scenario, steps, expected vs actual results, environment details, and SQL checks.
2. **Reproduce** — Run the failing test against the current implementation.
3. **Fix if needed** — Implement a minimal code fix scoped to the ticket.
4. **Reset environment** — Always. Run `./scripts/reset-docker.sh` to get a clean state.
5. **Re-run QA steps** — Follow the ticket's exact steps and SQL validation. Capture evidence.
6. **Analyze false positives** — This is the nuance that makes it work.
7. **Close ticket** — Delete the ticket file, summarize the outcome.

### False-Positive Analysis

Not every failed test is a bug. The `ticket-fix` skill explicitly handles false positives — failures caused by flawed test procedures rather than code defects:

- Test commands missing required authentication headers (e.g., HMAC signatures)
- Prerequisites that are incomplete or ambiguous ("webhook must exist" without creation steps)
- Environment assumptions that don't match the Docker default configuration
- Test data referencing non-existent entities without fallback handling

When a false positive is confirmed, the skill doesn't just close the ticket — it **updates the QA document** to prevent the same false positive from recurring. It makes implicit requirements explicit, ensures example commands are copy-paste-ready, and adds troubleshooting tables for common failure modes.

This means the test suite gets *better* every time a test fails, regardless of whether the failure was a real bug or a flawed test. This is the essence of "polishing" — through repeated verification and correction, both the software and the test documents become more refined.

## Documentation Governance

Test documents rot. Routes change, APIs evolve, permissions get restructured — and suddenly half your test suite is testing behavior that no longer exists.

Since all QA test documents are repeatedly executed by agents, if documents become outdated, we notice a large number of false-positive ticket reports. The best strategy to keep documents updated is to keep using them.

The `qa-doc-governance` skill runs periodic audits with a severity classification:

- **P0**: Broken navigability or unusable test flow (missing checklists, misleading auth steps, index drift)
- **P1**: Governance drift (>5 scenarios per file, missing UI entry visibility in UI-facing docs)
- **P2**: Style consistency (naming alignment, wording normalization)

The workflow: lint all docs → classify findings → remediate by priority → sync indexes and manifests → validate and report. It can run after any new feature or refactor, or be scheduled regularly (e.g., weekly), before major releases, and after incidents.

One critical rule: **every document must include a regression checklist**. No exceptions. This ensures that even if a document drifts, the checklist provides a minimum viable verification path.

## Cross-Doc Impact Analysis

When a feature changes, it doesn't just affect one test document. The `qa-doc-gen` skill includes a mandatory cross-doc impact analysis that scans all 156 documents for:

- Route changes (`/dashboard`, `/tenant/select`, `/auth/callback`)
- Token model changes (`id_token`, `tenant token`, `token exchange`)
- Permission/auth changes (`401`, `403`, `scope`, `audience`, `role`)
- UI navigation text and expected redirect paths

For each impacted document, the skill classifies the impact:

1. **Patch required** — Update steps, expected results, or security assertions
2. **Note required** — Add prerequisite or branch-path note to avoid tester confusion
3. **No change** — Explicitly record why unaffected

The key rule: **never stop at creating only new docs when old docs are stale.**

## What the Human Actually Does

This is human-AI collaboration, not replacement. Here's what I actually do day-to-day:

- **Planning**: Decide what to build, define acceptance criteria, choose architectural tradeoffs. The AI scaffolds; I steer.
- **Reviewing**: I review generated test documents and the first version of new feature / refactor code. QA execution and ticket-fix cycles? Those run on AI autonomously. The review effort concentrates where it matters most — the *specs* that define correctness, and the *initial implementation* that sets the direction.
- **Steering**: When the pipeline produces a false positive, I review the root cause analysis. When governance flags a P0, I decide the remediation approach.
- **Architecture**: Domain modeling, data flow design, security boundaries — these require human judgment about the *system*, not just the code.

In this project, I invested the same amount of personal time but achieved over 10x the output.

This doesn't mean you can let AI handle everything unsupervised. After 20 rounds of iteration, AI-executed tests still produce tickets — but far fewer than the early rounds, and the application gets richer in detail with each pass. The experience is remarkably similar to what human engineers do: we polish software through repeated QA, eliminating bugs layer by layer. The difference is that the polishing loop now runs faster, and every round is documented.

**The human's core value lies in defining "what we want to do and what we don't want to do"** and providing good enough taste and judgment. The AI is responsible for efficiently implementing this definition and continuously verifying and correcting it through automated testing.

A key insight: the role of human experts actually becomes more important — we need to create an environment where Agents can perform verification, which requires human experts to be proficient not only in development but also in infrastructure. Furthermore, the pursuit of DevOps and security issues helps human experts review documents more comprehensively and continuously improve this process. In short, we need true full-stack engineers.

As a developer, I've always advocated for extreme programming. As a tech lead, I trust my team members, but I leverage agile development methodologies, including test-driven practices, for risk management as much as possible. So when it comes to AI, my perspective is quite open: I believe almost all risk management techniques used in software development, especially extreme programming practices, can be applied to managing Agents.

## Honest Limitations

This approach isn't magic. Here's what doesn't work well yet:

- **Novel architecture decisions** — The AI is excellent at implementing patterns it's seen before. For genuinely novel design choices (our Token Exchange flow, for instance), human architects still lead.
- **Security review** — The AI generates security test documents and executes them, but the threat model itself requires human security expertise. Automated testing catches known patterns; it doesn't discover novel attack vectors.
- **Diminishing returns on governance** — At some point, maintaining 156 test documents has overhead that approaches the overhead of not having them. We haven't hit that point yet, but the curve isn't linear.
- **Process needs project adaptation** — The process itself needs optimization for specific projects. For example, some projects may not need UI testing, while others may focus more on API or data layer validation. The skill combinations and testing focus need adjustment based on project characteristics.

## By the Numbers

- **16** Agent Skills covering the full development lifecycle
- **156** test documents (96 QA + 48 security + 12 UI/UX)
- **9** tool scripts for token generation, API testing, gRPC smoke tests
- **~2,300** lines of skill definitions
- **1** human

## Why IAM?

A common question: why build an identity platform to test a development methodology?

Because easy problems don't test methodologies. A TODO app or a blog engine would prove nothing — any approach looks good when the domain is trivial. IAM forces you to deal with:

- **Security that can't be faked** — Token validation, permission enforcement, injection prevention. The 48 security test documents exist because "it works on my machine" isn't acceptable for authentication.
- **Multi-system coordination** — Keycloak, TiDB, Redis, the Rust backend, the React frontend. A bug in the token exchange flow touches every layer.
- **State complexity** — Multi-tenant data isolation, hierarchical RBAC, session management. The state space is large enough that manual testing can't cover it.

If AI-native SDLC can produce a polished IAM platform, it can work for most applications.

## Try It Yourself

The entire pipeline is open source: [github.com/gpgkd906/auth9](https://github.com/gpgkd906/auth9)

The `.agents/skills/` directory contains all 16 Agent Skills. The `docs/qa/`, `docs/security/`, and `docs/uiux/` directories contain the test documents. The `project-bootstrap` skill can scaffold a new project with the same structure — Rust backend, React frontend, Docker compose, Kubernetes manifests, and deployment scripts — so you can replicate this approach for your own projects.

The methodology isn't specific to identity platforms. It's a way of thinking about AI-led development: **don't just use AI to write code faster — use it to close the loop between planning, testing, fixing, and deploying.**

---

*Auth9 is a self-hosted identity and access management platform (Auth0 alternative) built with Rust, React Router 7, TiDB, and Keycloak. The AI-native development pipeline uses an Agent Skills architecture with 16 custom skills.*
