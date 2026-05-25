//! Direct Gmail REST API client. Replaces the Composio slugs
//! `GMAIL_SEND_EMAIL`, `GMAIL_FETCH_EMAILS`, `GMAIL_LIST_LABELS`,
//! `GMAIL_FETCH_MESSAGE_BY_MESSAGE_ID`, `GMAIL_DELETE_EMAIL`,
//! `GMAIL_MOVE_TO_TRASH`, `GMAIL_ADD_LABEL_TO_EMAIL` that the Composio backend previously
//! brokered. Endpoint reference:
//! <https://developers.google.com/gmail/api/reference/rest>.

use anyhow::{anyhow, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::openhuman::credentials::AuthService;
use crate::openhuman::oauth::persistence::GOOGLE_PROVIDER;

use crate::openhuman::providers_native::bearer::AuthedClient;

/// Google REST base; `users/me/...` references the authenticated user.
const BASE_URL: &str = "https://gmail.googleapis.com/gmail/v1";

/// Minimal pieces of a Gmail message we care about for the agent's
/// "summarise / draft a reply" surface. Add fields as actual call
/// sites need them rather than pulling the entire Message resource.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MessageId {
    pub id: String,
    #[serde(default, alias = "threadId")]
    pub thread_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ListResponse {
    #[serde(default)]
    pub messages: Vec<MessageId>,
    #[serde(default, alias = "resultSizeEstimate")]
    pub result_size_estimate: Option<u64>,
    #[serde(default, alias = "nextPageToken")]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Label {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, alias = "messageListVisibility")]
    pub message_list_visibility: Option<String>,
    #[serde(default, alias = "labelListVisibility")]
    pub label_list_visibility: Option<String>,
    #[serde(default, alias = "type")]
    pub label_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ListLabelsResponse {
    #[serde(default)]
    pub labels: Vec<Label>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GmailMessage {
    pub id: String,
    #[serde(default, alias = "threadId")]
    pub thread_id: Option<String>,
    #[serde(default, alias = "labelIds")]
    pub label_ids: Vec<String>,
    #[serde(default, alias = "historyId")]
    pub history_id: Option<String>,
    #[serde(default)]
    pub snippet: Option<String>,
    #[serde(default)]
    pub payload: Option<serde_json::Value>,
    #[serde(default)]
    pub raw: Option<String>,
}

/// Send a plain-text email as the authenticated user. Equivalent to
/// the Composio `GMAIL_SEND_EMAIL` slug.
///
/// `recipient` is a single RFC 5322 address. `subject` and `body` are
/// plain text — Gmail will accept a richer MIME tree, but the
/// production call site (issue #-tracked) only ever sent plain text,
/// so the API is narrowed accordingly.
pub async fn send_message(
    http: &reqwest::Client,
    service: &AuthService,
    recipient: &str,
    subject: &str,
    body: &str,
) -> Result<MessageId> {
    if recipient.trim().is_empty() {
        return Err(anyhow!("send_message: recipient must not be empty"));
    }
    let raw = build_raw_message(recipient, subject, body);
    let client = AuthedClient::new(http, service, GOOGLE_PROVIDER);
    let url = format!("{BASE_URL}/users/me/messages/send");
    let resp: MessageId = client.post_json(&url, &json!({ "raw": raw })).await?;
    Ok(resp)
}

/// List Gmail messages matching `query` (the same syntax accepted by
/// the Gmail UI search box). `max_results` is clamped at the API
/// max of 500. Equivalent to `GMAIL_FETCH_EMAILS`.
pub async fn list_messages(
    http: &reqwest::Client,
    service: &AuthService,
    query: Option<&str>,
    max_results: u32,
) -> Result<ListResponse> {
    let client = AuthedClient::new(http, service, GOOGLE_PROVIDER);
    let max_results = max_results.clamp(1, 500);
    let mut url = format!("{BASE_URL}/users/me/messages?maxResults={max_results}");
    if let Some(q) = query {
        url.push_str("&q=");
        url.push_str(&urlencoding(q));
    }
    client.get_json::<ListResponse>(&url).await
}

/// List labels for the authenticated mailbox. Equivalent to
/// `GMAIL_LIST_LABELS`.
pub async fn list_labels(
    http: &reqwest::Client,
    service: &AuthService,
) -> Result<ListLabelsResponse> {
    let client = AuthedClient::new(http, service, GOOGLE_PROVIDER);
    let url = format!("{BASE_URL}/users/me/labels");
    client.get_json::<ListLabelsResponse>(&url).await
}

/// Fetch one Gmail message by id. Equivalent to
/// `GMAIL_FETCH_MESSAGE_BY_MESSAGE_ID`.
pub async fn get_message(
    http: &reqwest::Client,
    service: &AuthService,
    message_id: &str,
    format: Option<&str>,
) -> Result<GmailMessage> {
    if message_id.trim().is_empty() {
        return Err(anyhow!("get_message: message_id must not be empty"));
    }
    let client = AuthedClient::new(http, service, GOOGLE_PROVIDER);
    let format = validated_message_format(format)?;
    let url = format!(
        "{BASE_URL}/users/me/messages/{}?format={format}",
        urlencoding(message_id)
    );
    client.get_json::<GmailMessage>(&url).await
}

/// Permanently delete a message by ID. Equivalent to
/// `GMAIL_DELETE_EMAIL`.
pub async fn delete_message(
    http: &reqwest::Client,
    service: &AuthService,
    message_id: &str,
) -> Result<()> {
    let client = AuthedClient::new(http, service, GOOGLE_PROVIDER);
    let url = format!("{BASE_URL}/users/me/messages/{}", urlencoding(message_id));
    client.delete(&url).await
}

/// Move a message to the user's trash. Equivalent to
/// `GMAIL_MOVE_TO_TRASH`.
pub async fn trash_message(
    http: &reqwest::Client,
    service: &AuthService,
    message_id: &str,
) -> Result<GmailMessage> {
    if message_id.trim().is_empty() {
        return Err(anyhow!("trash_message: message_id must not be empty"));
    }
    let client = AuthedClient::new(http, service, GOOGLE_PROVIDER);
    let url = format!(
        "{BASE_URL}/users/me/messages/{}/trash",
        urlencoding(message_id)
    );
    client.post_json::<GmailMessage>(&url, &json!({})).await
}

/// Add a label to an existing message. Equivalent to
/// `GMAIL_ADD_LABEL_TO_EMAIL`. The label must already exist — Gmail
/// will error with HTTP 400 otherwise.
pub async fn add_label(
    http: &reqwest::Client,
    service: &AuthService,
    message_id: &str,
    label_id: &str,
) -> Result<MessageId> {
    let client = AuthedClient::new(http, service, GOOGLE_PROVIDER);
    let url = format!(
        "{BASE_URL}/users/me/messages/{}/modify",
        urlencoding(message_id)
    );
    let body = json!({ "addLabelIds": [label_id] });
    client.post_json::<MessageId>(&url, &body).await
}

/// Encode the RFC 5322 message as URL-safe base64 with padding stripped
/// — the wire format Gmail expects in the `raw` field. Pulled out so
/// it can be unit-tested without spinning up the full client.
pub(crate) fn build_raw_message(to: &str, subject: &str, body: &str) -> String {
    let mime = format!(
        "To: {to}\r\nSubject: {subject}\r\nMIME-Version: 1.0\r\n\
         Content-Type: text/plain; charset=utf-8\r\n\r\n{body}"
    );
    URL_SAFE_NO_PAD.encode(mime.as_bytes())
}

/// Tiny URL-encoder for query-string values. `urlencoding` is not a
/// direct dep, so we percent-encode just the characters that matter
/// for Gmail search queries.
fn urlencoding(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(byte as char);
            }
            _ => {
                out.push('%');
                out.push_str(&format!("{byte:02X}"));
            }
        }
    }
    out
}

fn validated_message_format(format: Option<&str>) -> Result<&'static str> {
    match format
        .unwrap_or("full")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "minimal" => Ok("minimal"),
        "metadata" => Ok("metadata"),
        "full" => Ok("full"),
        "raw" => Ok("raw"),
        other => Err(anyhow!(
            "get_message: unsupported Gmail message format '{other}'"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_raw_message_is_base64url_no_pad() {
        let raw = build_raw_message("alice@example.com", "Hi", "Hello!");
        assert!(!raw.contains('='), "raw must be padding-free: {raw}");
        assert!(
            !raw.contains('+') && !raw.contains('/'),
            "must be url-safe: {raw}"
        );
        // Decode and verify the MIME body round-trips.
        let decoded = URL_SAFE_NO_PAD.decode(raw.as_bytes()).unwrap();
        let mime = String::from_utf8(decoded).unwrap();
        assert!(mime.contains("To: alice@example.com\r\n"));
        assert!(mime.contains("Subject: Hi\r\n"));
        assert!(mime.contains("\r\n\r\nHello!"));
    }

    #[test]
    fn urlencoding_percent_encodes_spaces_and_punctuation() {
        assert_eq!(
            urlencoding("from:bob@example.com"),
            "from%3Abob%40example.com"
        );
        assert_eq!(urlencoding("subject:Q3 plan"), "subject%3AQ3%20plan");
    }

    #[test]
    fn gmail_responses_accept_camel_case_fields() {
        let list: ListResponse = serde_json::from_value(json!({
            "messages": [{ "id": "m1", "threadId": "t1" }],
            "resultSizeEstimate": 1,
            "nextPageToken": "next"
        }))
        .unwrap();
        assert_eq!(list.messages[0].thread_id.as_deref(), Some("t1"));
        assert_eq!(list.result_size_estimate, Some(1));
        assert_eq!(list.next_page_token.as_deref(), Some("next"));

        let message: GmailMessage = serde_json::from_value(json!({
            "id": "m1",
            "threadId": "t1",
            "labelIds": ["INBOX"],
            "historyId": "42",
            "snippet": "hello"
        }))
        .unwrap();
        assert_eq!(message.thread_id.as_deref(), Some("t1"));
        assert_eq!(message.label_ids, vec!["INBOX"]);
        assert_eq!(message.history_id.as_deref(), Some("42"));
    }

    #[test]
    fn validated_message_format_defaults_to_full_and_rejects_unknown() {
        assert_eq!(validated_message_format(None).unwrap(), "full");
        assert_eq!(
            validated_message_format(Some("metadata")).unwrap(),
            "metadata"
        );
        let err = validated_message_format(Some("summary")).unwrap_err();
        assert!(err.to_string().contains("unsupported Gmail message format"));
    }

    #[test]
    fn send_message_rejects_empty_recipient() {
        // The guard is sync, but the function is async; build a tiny
        // runtime to drive it without standing up a server.
        let rt = tokio::runtime::Runtime::new().unwrap();
        let dir = tempfile::TempDir::new().unwrap();
        let svc = AuthService::new(dir.path(), true);
        let http = reqwest::Client::new();
        let err = rt
            .block_on(send_message(&http, &svc, "", "subj", "body"))
            .unwrap_err();
        assert!(err.to_string().contains("recipient must not be empty"));
    }
}
