//! Integration tests for `POST /v1/search`.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};

#[tokio::test]
async fn search_returns_results_for_a_text_query() {
    let app = common::seeded_app();
    let body = serde_json::json!({
        "text": "instruct",
        "limit": 10,
    });
    let req = Request::builder()
        .method("POST")
        .uri("/v1/search")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .expect("build request");

    let (resp, body) = common::send(app.router, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let json: serde_json::Value = serde_json::from_slice(&body).expect("parse body");
    assert_eq!(json["query"], "instruct");
    assert_eq!(json["limit"], 10);

    let results = json["results"].as_array().expect("results array");
    assert!(!results.is_empty(), "expected at least one BM25 hit for `instruct`");
    for r in results {
        assert!(r["id"].is_string());
        assert!(r["score"].is_number());
    }
}

#[tokio::test]
async fn search_applies_kinds_facet_filter() {
    let app = common::seeded_app();
    // Every seeded row is `instruct`; ask for `instruct` + `coding` and
    // verify we still get results back (the facet OR-matches).
    let body = serde_json::json!({
        "text": "instruct",
        "kinds": ["instruct", "coding"],
        "limit": 10,
    });
    let req = Request::builder()
        .method("POST")
        .uri("/v1/search")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .expect("build request");

    let (resp, body) = common::send(app.router, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let json: serde_json::Value = serde_json::from_slice(&body).expect("parse body");
    let results = json["results"].as_array().expect("results array");
    assert!(!results.is_empty(), "kinds facet must not silently drop all rows");
}

#[tokio::test]
async fn search_with_empty_text_returns_no_results() {
    let app = common::seeded_app();
    let body = serde_json::json!({ "text": "", "limit": 10 });
    let req = Request::builder()
        .method("POST")
        .uri("/v1/search")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .expect("build request");

    let (resp, body) = common::send(app.router, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json: serde_json::Value = serde_json::from_slice(&body).expect("parse body");
    assert_eq!(
        json["results"].as_array().expect("results").len(),
        0,
        "empty text must yield zero results"
    );
}

#[tokio::test]
async fn search_with_unknown_kind_does_not_500() {
    let app = common::seeded_app();
    // A bogus kind entry is silently dropped by `parse_kind`; the request
    // must still succeed with the empty kinds facet applied.
    let body = serde_json::json!({
        "text": "instruct",
        "kinds": ["definitely_not_a_kind"],
        "limit": 5,
    });
    let req = Request::builder()
        .method("POST")
        .uri("/v1/search")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .expect("build request");

    let (resp, _body) = common::send(app.router, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}