//! Content shared by the Responses request, response and streaming adapters.
//! Unrepresentable media is rejected instead of disappearing from a conversation.
use crate::openai_chat::ReasoningSigner;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{json, Value};

const PURPOSE: &str = "csswitch.openai-responses-thinking.v1";
const MAX_MEDIA: usize = 8 * 1024 * 1024;

pub(crate) fn image(block: &Value) -> Result<Value, String> {
    let source = &block["source"];
    let image_url = match source["type"].as_str() {
        Some("base64") => {
            let mime = source["media_type"]
                .as_str()
                .filter(|m| matches!(*m, "image/png" | "image/jpeg" | "image/gif" | "image/webp"))
                .ok_or("unsupported image media type")?;
            let data = source["data"]
                .as_str()
                .filter(|d| !d.is_empty() && d.len() <= MAX_MEDIA)
                .ok_or("image exceeds the limit or has no data")?;
            STANDARD
                .decode(data)
                .map_err(|_| "image data is not valid base64")?;
            format!("data:{mime};base64,{data}")
        }
        Some("url") => {
            let raw = source["url"].as_str().ok_or("image URL is missing")?;
            let url = url::Url::parse(raw).map_err(|_| "image URL is invalid")?;
            if url.scheme() != "https"
                || url.host_str().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
                || url.fragment().is_some()
                || raw.len() > 8192
            {
                return Err("image URL must use HTTPS without credentials or a fragment".into());
            }
            raw.to_string()
        }
        _ => return Err("image source is unsupported".into()),
    };
    Ok(json!({"type":"input_image","image_url":image_url}))
}

pub(crate) fn reasoning_text(item: &Value) -> Result<String, String> {
    let mut text = String::new();
    for (field, expected) in [("summary", "summary_text"), ("content", "reasoning_text")] {
        if let Some(parts) = item.get(field) {
            for part in parts
                .as_array()
                .ok_or("Responses reasoning parts are invalid")?
            {
                if part["type"] != expected {
                    return Err("Responses reasoning part is unsupported".into());
                }
                let part = part["text"]
                    .as_str()
                    .ok_or("Responses reasoning text is invalid")?;
                if text.len().saturating_add(part.len()) > 1024 * 1024 {
                    return Err("Responses reasoning exceeds the limit".into());
                }
                text.push_str(part);
            }
        }
    }
    Ok(text)
}

pub(crate) fn reasoning_block(
    item: &Value,
    target: &str,
    signer: &ReasoningSigner,
) -> Result<Value, String> {
    let text = reasoning_text(item)?;
    let signature = signer.seal_payload(PURPOSE, &json!({"model":target,"item":item}))?;
    Ok(if text.is_empty() {
        json!({"type":"redacted_thinking","data":signature})
    } else {
        json!({"type":"thinking","thinking":text,"signature":signature})
    })
}

pub(crate) fn restore_reasoning(
    block: &Value,
    target: &str,
    signer: &ReasoningSigner,
) -> Result<Value, String> {
    let signature = block[if block["type"] == "thinking" {
        "signature"
    } else {
        "data"
    }]
    .as_str()
    .ok_or("Responses reasoning signature is missing")?;
    let envelope = signer.verify_payload(PURPOSE, signature)?;
    if envelope["model"] != target || envelope["item"]["type"] != "reasoning" {
        return Err("Responses reasoning context changed".into());
    }
    let text = reasoning_text(&envelope["item"])?;
    if (block["type"] == "thinking" && block["thinking"].as_str() != Some(text.as_str()))
        || (block["type"] == "redacted_thinking" && !text.is_empty())
    {
        return Err("Responses reasoning content changed".into());
    }
    Ok(envelope["item"].clone())
}

pub(crate) fn validate_output(resp: &Value) -> Result<(), String> {
    if !resp.is_object() || resp.get("error").is_some_and(|e| !e.is_null()) {
        return Err("Responses upstream failed".into());
    }
    if resp
        .get("status")
        .is_some_and(|s| !matches!(s.as_str(), Some("completed" | "incomplete")))
    {
        return Err("Responses status is unsupported".into());
    }
    if !resp.get("output").is_some_and(Value::is_array)
        && !resp.get("output_text").is_some_and(Value::is_string)
    {
        return Err("Responses output is missing or invalid".into());
    }
    if resp.get("output").is_some_and(|v| !v.is_array())
        || resp.get("output_text").is_some_and(|v| !v.is_string())
    {
        return Err("Responses output fields are invalid".into());
    }
    if resp["status"] == "incomplete"
        && resp.get("incomplete_details").is_some()
        && resp["incomplete_details"]["reason"] != "max_output_tokens"
    {
        return Err("Responses incomplete reason is unsupported".into());
    }
    for item in resp
        .get("output")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if item.get("status").is_some_and(|status| {
            status != "completed" && !(resp["status"] == "incomplete" && status == "incomplete")
        }) {
            return Err("Responses output item is unfinished".into());
        }
        match item["type"].as_str() {
            Some("message") => {
                if item.get("role").is_some_and(|role| role != "assistant") {
                    return Err("Responses output role is invalid".into());
                }
                for part in item["content"]
                    .as_array()
                    .ok_or("Responses message content is invalid")?
                {
                    if !matches!(part["type"].as_str(), Some("output_text" | "text"))
                        || !part["text"].is_string()
                    {
                        return Err("Responses output media or refusal cannot be represented by this transport".into());
                    }
                }
            }
            Some("reasoning") => {
                reasoning_text(item)?;
            }
            Some("function_call") => {}
            _ => {
                return Err("Responses output item cannot be represented by this transport".into())
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn image_sources_are_translated_and_invalid_media_rejected() {
        assert_eq!(
            image(&json!({"source":{"type":"base64","media_type":"image/png","data":"aW1hZ2U="}}))
                .unwrap(),
            json!({"type":"input_image","image_url":"data:image/png;base64,aW1hZ2U="})
        );
        assert!(
            image(&json!({"source":{"type":"base64","media_type":"image/png","data":"%%%"}}))
                .is_err()
        );
        assert!(
            image(&json!({"source":{"type":"url","url":"https://secret@example.invalid/a"}}))
                .is_err()
        );
        assert!(image(&json!({"source":{"type":"url","url":"file:///tmp/a"}})).is_err());
    }
    #[test]
    fn reasoning_round_trip_preserves_opaque_state_and_rejects_tampering() {
        let signer = ReasoningSigner::new("synthetic", "fixture-profile").unwrap();
        let item = json!({"type":"reasoning","id":"r1","summary":[{"type":"summary_text","text":"Inspect the fixture"}],"encrypted_content":"opaque-fixture"});
        let block = reasoning_block(&item, "fixture-model", &signer).unwrap();
        assert_eq!(block["type"], "thinking");
        assert_eq!(
            restore_reasoning(&block, "fixture-model", &signer).unwrap(),
            item
        );
        assert!(restore_reasoning(&block, "different-model", &signer).is_err());
        let other = ReasoningSigner::new("synthetic", "different-profile").unwrap();
        assert!(restore_reasoning(&block, "fixture-model", &other).is_err());
        let mut tampered = block.clone();
        tampered["thinking"] = json!("changed");
        assert!(restore_reasoning(&tampered, "fixture-model", &signer).is_err());
        let mut forged = block;
        forged["signature"] = json!("csswitch.openai-responses-thinking.v1.e30.invalid");
        assert!(restore_reasoning(&forged, "fixture-model", &signer).is_err());
    }
    #[test]
    fn opaque_reasoning_without_summary_uses_redacted_thinking() {
        let signer = ReasoningSigner::new("synthetic", "fixture-profile").unwrap();
        let item =
            json!({"type":"reasoning","id":"r1","summary":[],"encrypted_content":"opaque-fixture"});
        let block = reasoning_block(&item, "fixture-model", &signer).unwrap();
        assert_eq!(block["type"], "redacted_thinking");
        assert_eq!(
            restore_reasoning(&block, "fixture-model", &signer).unwrap(),
            item
        );
    }
    #[test]
    fn output_images_audio_refusals_and_failed_responses_are_not_silently_lost() {
        for part in [
            json!({"type":"output_audio","data":"fixture"}),
            json!({"type":"image","url":"fixture"}),
            json!({"type":"refusal","refusal":"fixture"}),
        ] {
            assert!(validate_output(
                &json!({"status":"completed","output":[{"type":"message","content":[part]}]})
            )
            .is_err());
        }
        assert!(validate_output(&json!({"status":"failed","output":[]})).is_err());
        assert!(validate_output(&json!({})).is_err());
    }
    #[test]
    fn malformed_output_fields_and_unfinished_items_cannot_be_success() {
        for response in [
            json!({"status":"completed","output":"broken","output_text":"apparently valid"}),
            json!({"status":"completed","output":[{"type":"message","role":"user","content":[{"type":"output_text","text":"user text"}]}]}),
            json!({"status":"completed","output":[{"type":"message","status":"in_progress","content":[{"type":"output_text","text":"partial"}]}]}),
        ] {
            assert!(
                validate_output(&response).is_err(),
                "invalid response was accepted: {response}"
            );
        }
    }
}
