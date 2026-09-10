use std::fmt::Write as _;

#[derive(Clone, Copy)]
pub struct Method(pub &'static str);

#[derive(Clone)]
pub struct MockEndpoint {
    pub method: Method,
    pub path: &'static str,
    pub status: u16,
    pub latency_ms: u32,
    pub request_body: &'static str,
    pub response_body: &'static str,
    pub summary: &'static str,
}

#[must_use]
pub fn catalog() -> Vec<MockEndpoint> {
    let mut out = users_endpoints();
    out.extend(projects_endpoints());
    out.extend(billing_endpoints());
    out
}

fn users_endpoints() -> Vec<MockEndpoint> {
    vec![
        MockEndpoint {
            method: Method("GET"),
            path: "/v1/users",
            status: 200,
            latency_ms: 38,
            summary: "List all users in the workspace.",
            request_body: "(no body — query params: ?page=1&limit=25)",
            response_body: "{\n  \"users\": [\n    { \"id\": 1, \"name\": \"ada\" },\n    { \"id\": 2, \"name\": \"grace\" }\n  ],\n  \"page\": 1,\n  \"total\": 2\n}\n",
        },
        MockEndpoint {
            method: Method("POST"),
            path: "/v1/users",
            status: 201,
            latency_ms: 84,
            summary: "Create a new user.",
            request_body: "{\n  \"name\": \"new-user\",\n  \"email\": \"new@example.dev\"\n}\n",
            response_body: "{\n  \"id\": 3,\n  \"name\": \"new-user\",\n  \"email\": \"new@example.dev\",\n  \"created_at\": \"2026-05-20T11:42:00Z\"\n}\n",
        },
        MockEndpoint {
            method: Method("GET"),
            path: "/v1/users/{id}",
            status: 200,
            latency_ms: 22,
            summary: "Fetch one user by id.",
            request_body: "(no body)",
            response_body: "{\n  \"id\": 1,\n  \"name\": \"ada\",\n  \"email\": \"ada@example.dev\",\n  \"role\": \"owner\"\n}\n",
        },
        MockEndpoint {
            method: Method("DELETE"),
            path: "/v1/users/{id}",
            status: 204,
            latency_ms: 41,
            summary: "Soft-delete a user.",
            request_body: "(no body)",
            response_body: "(empty — 204 No Content)\n",
        },
    ]
}

fn projects_endpoints() -> Vec<MockEndpoint> {
    vec![
        MockEndpoint {
            method: Method("GET"),
            path: "/v1/projects",
            status: 200,
            latency_ms: 52,
            summary: "List projects in the workspace.",
            request_body: "(no body)",
            response_body: "{\n  \"projects\": [\n    { \"id\": \"atlas\", \"name\": \"Atlas\" },\n    { \"id\": \"sigil\", \"name\": \"Sigil\" }\n  ]\n}\n",
        },
        MockEndpoint {
            method: Method("POST"),
            path: "/v1/projects/{id}/deploy",
            status: 202,
            latency_ms: 312,
            summary: "Kick off an async deploy.",
            request_body: "{\n  \"environment\": \"production\",\n  \"branch\": \"main\"\n}\n",
            response_body: "{\n  \"job_id\": \"dep-2026-05-20-001\",\n  \"status\": \"queued\"\n}\n",
        },
        MockEndpoint {
            method: Method("GET"),
            path: "/v1/projects/{id}/builds",
            status: 200,
            latency_ms: 71,
            summary: "List recent CI builds.",
            request_body: "(no body)",
            response_body: "{\n  \"builds\": [\n    { \"id\": 412, \"status\": \"green\" },\n    { \"id\": 411, \"status\": \"red\"   }\n  ]\n}\n",
        },
    ]
}

fn billing_endpoints() -> Vec<MockEndpoint> {
    vec![
        MockEndpoint {
            method: Method("GET"),
            path: "/v1/billing/invoices",
            status: 200,
            latency_ms: 96,
            summary: "List invoices for the current workspace.",
            request_body: "(no body)",
            response_body: "{\n  \"invoices\": [\n    { \"id\": \"inv-2026-05\", \"total_cents\": 1900 }\n  ]\n}\n",
        },
        MockEndpoint {
            method: Method("POST"),
            path: "/v1/billing/portal",
            status: 200,
            latency_ms: 144,
            summary: "Create a Stripe customer-portal session.",
            request_body: "{\n  \"return_url\": \"https://app.example.dev/settings/billing\"\n}\n",
            response_body: "{\n  \"url\": \"https://billing.stripe.com/p/session/test_abc123\"\n}\n",
        },
        MockEndpoint {
            method: Method("GET"),
            path: "/v1/billing/usage",
            status: 200,
            latency_ms: 63,
            summary: "Aggregated usage for the current period.",
            request_body: "(no body)",
            response_body: "{\n  \"period\": \"2026-05\",\n  \"requests\": 124812,\n  \"bandwidth_gb\": 38.4\n}\n",
        },
        MockEndpoint {
            method: Method("PUT"),
            path: "/v1/billing/plan",
            status: 200,
            latency_ms: 118,
            summary: "Switch the workspace to a new plan.",
            request_body: "{\n  \"plan\": \"pro-monthly\"\n}\n",
            response_body: "{\n  \"plan\": \"pro-monthly\",\n  \"prorated_cents\": 412\n}\n",
        },
        MockEndpoint {
            method: Method("DELETE"),
            path: "/v1/billing/subscription",
            status: 202,
            latency_ms: 88,
            summary: "Cancel the active subscription at period end.",
            request_body: "(no body)",
            response_body: "{\n  \"status\": \"cancel_at_period_end\",\n  \"effective\": \"2026-06-01\"\n}\n",
        },
    ]
}

#[must_use]
pub fn header_block(endpoint: &MockEndpoint) -> String {
    let mut out = String::with_capacity(256);
    let _ = writeln!(out, "HTTP/2 {}", endpoint.status);
    out.push_str("content-type: application/json; charset=utf-8\n");
    let _ = writeln!(out, "x-request-id: req-{:08x}", path_seed(endpoint.path));
    let _ = writeln!(out, "x-response-time-ms: {}", endpoint.latency_ms);
    out.push_str("cache-control: no-store\n");
    out.push_str("x-served-by: edge-iad\n");
    out
}

#[must_use]
pub fn response_summary(endpoint: &MockEndpoint) -> String {
    format!(
        "{} {}\n  status:  {}\n  latency: {} ms\n\n",
        endpoint.method.0, endpoint.path, endpoint.status, endpoint.latency_ms
    )
}

fn path_seed(path: &str) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for &byte in path.as_bytes() {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}
