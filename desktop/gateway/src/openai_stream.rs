//! Incremental OpenAI text SSE with bounded framing and validated tool completion.
//! Tool arguments and signed reasoning are released after final validation.
use crate::{openai_chat, openai_responses};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

const FRAME_LIMIT: usize = 1024 * 1024;
const TOTAL_LIMIT: usize = 16 * 1024 * 1024;
type Result<T> = std::result::Result<T, &'static str>;
pub(crate) type Event = (String, Value);
pub(crate) enum Frame {
    Json(Value),
    Done,
}
#[derive(Default)]
pub(crate) struct Decoder {
    pending: Vec<u8>,
    total: usize,
}
impl Decoder {
    pub(crate) fn feed(&mut self, bytes: &[u8]) -> Result<Vec<Frame>> {
        self.total = self.total.saturating_add(bytes.len());
        if self.total > TOTAL_LIMIT {
            return Err("OpenAI stream exceeds the response limit");
        }
        self.pending.extend_from_slice(bytes);
        let mut frames = Vec::new();
        loop {
            let end = self
                .pending
                .windows(2)
                .position(|p| p == b"\n\n")
                .map(|i| (i, 2));
            let crlf = self
                .pending
                .windows(4)
                .position(|p| p == b"\r\n\r\n")
                .map(|i| (i, 4));
            let Some((at, len)) = [end, crlf].into_iter().flatten().min_by_key(|p| p.0) else {
                break;
            };
            if at > FRAME_LIMIT {
                return Err("OpenAI SSE frame exceeds the limit");
            }
            let bytes: Vec<u8> = self.pending.drain(..at + len).collect();
            let frame = std::str::from_utf8(&bytes).map_err(|_| "OpenAI SSE is not UTF-8")?;
            let mut data = Vec::new();
            let mut event = None;
            for line in frame.lines() {
                if let Some(value) = line.strip_prefix("data:") {
                    data.push(value.strip_prefix(' ').unwrap_or(value));
                } else if let Some(value) = line.strip_prefix("event:") {
                    if event.is_some() {
                        return Err("duplicate SSE event");
                    }
                    event = Some(value.trim());
                } else if !line.is_empty()
                    && !line.starts_with(':')
                    && !line.starts_with("id:")
                    && !line.starts_with("retry:")
                {
                    return Err("invalid SSE field");
                }
            }
            if data.is_empty() {
                continue;
            }
            let data = data.join("\n");
            if data == "[DONE]" {
                frames.push(Frame::Done);
                continue;
            }
            let value: Value =
                serde_json::from_str(&data).map_err(|_| "OpenAI SSE JSON is invalid")?;
            if !value.is_object()
                || event.is_some_and(|e| value.get("type").and_then(Value::as_str) != Some(e))
            {
                return Err("OpenAI SSE event type mismatch");
            }
            frames.push(Frame::Json(value));
        }
        if self.pending.len() > FRAME_LIMIT {
            return Err("OpenAI SSE frame exceeds the limit");
        }
        Ok(frames)
    }
    pub(crate) fn finish(&self) -> Result<()> {
        if self.pending.iter().all(u8::is_ascii_whitespace) {
            Ok(())
        } else {
            Err("OpenAI SSE ended with a partial frame")
        }
    }
}
fn event(kind: &str, data: Value) -> Event {
    (kind.to_string(), data)
}
#[derive(Default)]
struct Emitter {
    started: bool,
    index: u64,
    text_open: bool,
    complete: bool,
    emitted_text: String,
}
impl Emitter {
    fn start(&mut self, id: &str, model: &str, out: &mut Vec<Event>) -> Result<()> {
        if self.started || id.is_empty() || id.len() > 256 {
            return Err("OpenAI stream start is invalid");
        }
        self.started = true;
        out.push(event("message_start", json!({"type":"message_start","message":{"id":id,"type":"message","role":"assistant","model":model,"content":[],"stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":0,"output_tokens":0}}})));
        Ok(())
    }
    fn text(&mut self, text: &str, out: &mut Vec<Event>) -> Result<()> {
        if !self.started || self.complete {
            return Err("OpenAI text lifecycle is invalid");
        }
        if text.is_empty() {
            return Ok(());
        }
        if !self.text_open {
            out.push(event("content_block_start", json!({"type":"content_block_start","index":self.index,"content_block":{"type":"text","text":""}})));
            self.text_open = true;
        }
        out.push(event("content_block_delta", json!({"type":"content_block_delta","index":self.index,"delta":{"type":"text_delta","text":text}})));
        self.emitted_text.push_str(text);
        Ok(())
    }
    fn finish(&mut self, message: &Value, out: &mut Vec<Event>) -> Result<()> {
        if !self.started || self.complete {
            return Err("duplicate OpenAI completion");
        }
        if self.text_open {
            out.push(event(
                "content_block_stop",
                json!({"type":"content_block_stop","index":self.index}),
            ));
            self.index += 1;
            self.text_open = false;
        }
        let emitted = self.emitted_text.clone();
        let mut remaining = emitted.as_str();
        for block in message["content"]
            .as_array()
            .ok_or("OpenAI content is invalid")?
        {
            let kind = block["type"]
                .as_str()
                .ok_or("OpenAI content type missing")?;
            if kind == "text" {
                let text = block["text"]
                    .as_str()
                    .ok_or("OpenAI text content invalid")?;
                let consumed = remaining.len().min(text.len());
                if !text.is_char_boundary(consumed)
                    || remaining.as_bytes()[..consumed] != text.as_bytes()[..consumed]
                {
                    return Err("OpenAI emitted text order changed");
                }
                remaining = &remaining[consumed..];
                self.text(&text[consumed..], out)?;
                if self.text_open {
                    out.push(event(
                        "content_block_stop",
                        json!({"type":"content_block_stop","index":self.index}),
                    ));
                    self.index += 1;
                    self.text_open = false;
                }
                continue;
            }
            let mut initial = block.clone();
            match kind {
                "tool_use" => initial["input"] = json!({}),
                "thinking" => {
                    initial["thinking"] = json!("");
                    initial.as_object_mut().unwrap().remove("signature");
                }
                "redacted_thinking" => {}
                _ => return Err("OpenAI content type unsupported"),
            }
            out.push(event(
                "content_block_start",
                json!({"type":"content_block_start","index":self.index,"content_block":initial}),
            ));
            if kind == "redacted_thinking" {
                out.push(event(
                    "content_block_stop",
                    json!({"type":"content_block_stop","index":self.index}),
                ));
                self.index += 1;
                continue;
            }
            let delta = if kind == "tool_use" {
                json!({"type":"input_json_delta","partial_json":serde_json::to_string(&block["input"]).map_err(|_| "invalid tool JSON")?})
            } else {
                json!({"type":"thinking_delta","thinking":block["thinking"]})
            };
            out.push(event(
                "content_block_delta",
                json!({"type":"content_block_delta","index":self.index,"delta":delta}),
            ));
            if kind == "thinking" {
                out.push(event("content_block_delta", json!({"type":"content_block_delta","index":self.index,"delta":{"type":"signature_delta","signature":block["signature"]}})));
            }
            out.push(event(
                "content_block_stop",
                json!({"type":"content_block_stop","index":self.index}),
            ));
            self.index += 1;
        }
        if !remaining.is_empty() {
            return Err("OpenAI emitted text was not in the completion");
        }
        out.push(event("message_delta", json!({"type":"message_delta","delta":{"stop_reason":message["stop_reason"],"stop_sequence":null},"usage":message["usage"]})));
        out.push(event("message_stop", json!({"type":"message_stop"})));
        self.complete = true;
        Ok(())
    }
}
#[derive(Default)]
struct Tool {
    id: String,
    name: String,
    args: String,
}
fn append(target: &mut String, value: &Value, limit: usize) -> Result<()> {
    let value = value.as_str().ok_or("OpenAI delta is not text")?;
    if target.len().saturating_add(value.len()) > limit {
        return Err("OpenAI delta exceeds the limit");
    }
    target.push_str(value);
    Ok(())
}
pub(crate) struct Chat<'a> {
    emitter: Emitter,
    id: String,
    text: String,
    reasoning: String,
    tools: BTreeMap<u64, Tool>,
    finish: Option<String>,
    usage: Value,
    model: &'a str,
    target: &'a str,
    signer: &'a openai_chat::ReasoningSigner,
    known: &'a Map<String, Value>,
}
impl<'a> Chat<'a> {
    pub(crate) fn new(
        model: &'a str,
        target: &'a str,
        signer: &'a openai_chat::ReasoningSigner,
        known: &'a Map<String, Value>,
    ) -> Self {
        Self {
            emitter: Emitter::default(),
            id: String::new(),
            text: String::new(),
            reasoning: String::new(),
            tools: BTreeMap::new(),
            finish: None,
            usage: Value::Null,
            model,
            target,
            signer,
            known,
        }
    }
    pub(crate) fn complete(&self) -> bool {
        self.emitter.complete
    }
    pub(crate) fn apply(&mut self, frame: Frame) -> Result<Vec<Event>> {
        if self.complete() {
            return Err("OpenAI data follows completion");
        }
        let mut out = Vec::new();
        let Frame::Json(value) = frame else {
            if self.finish.is_none() {
                return Err("OpenAI stream has no finish reason");
            }
            let calls: Vec<Value> = self.tools.values().map(|t| json!({"id":t.id,"type":"function","function":{"name":t.name,"arguments":t.args}})).collect();
            if self.tools.keys().copied().ne(0..self.tools.len() as u64) {
                return Err("OpenAI tool indices are sparse");
            }
            let response = json!({"id":self.id,"choices":[{"index":0,"message":{"role":"assistant","content":self.text,"reasoning_content":self.reasoning,"tool_calls":calls},"finish_reason":self.finish}],"usage":self.usage});
            let mapped = openai_chat::openai_to_anthropic_validated(
                &response,
                self.model,
                self.target,
                self.signer,
                self.known,
            )
            .map_err(|_| "OpenAI completion failed validation")?;
            self.emitter.finish(&mapped, &mut out)?;
            return Ok(out);
        };
        if value.get("error").is_some() {
            return Err("OpenAI upstream reported an error");
        }
        let id = value["id"].as_str().ok_or("OpenAI stream id missing")?;
        if !self.emitter.started {
            self.id = id.to_string();
            self.emitter.start(id, self.model, &mut out)?;
        } else if self.id != id {
            return Err("OpenAI stream id changed");
        }
        if let Some(usage) = value.get("usage").filter(|v| !v.is_null()) {
            self.usage = usage.clone();
        }
        let choices = value["choices"]
            .as_array()
            .ok_or("OpenAI choices missing")?;
        if choices.is_empty() {
            return Ok(out);
        }
        if choices.len() != 1 || choices[0]["index"].as_u64() != Some(0) || self.finish.is_some() {
            return Err("OpenAI choice lifecycle invalid");
        }
        let choice = &choices[0];
        let delta = choice["delta"].as_object().ok_or("OpenAI delta missing")?;
        if delta
            .get("role")
            .is_some_and(|v| v.as_str() != Some("assistant"))
            || delta.get("refusal").is_some_and(|v| !v.is_null())
            || delta.get("function_call").is_some()
            || delta.get("audio").is_some()
        {
            return Err("OpenAI delta type unsupported");
        }
        if let Some(reasoning) = delta.get("reasoning_content").filter(|v| !v.is_null()) {
            if reasoning.as_str().is_some_and(|r| !r.is_empty())
                && !self.emitter.emitted_text.is_empty()
            {
                return Err("OpenAI reasoning followed emitted text");
            }
            append(&mut self.reasoning, reasoning, FRAME_LIMIT)?;
        }
        if let Some(text) = delta.get("content").filter(|v| !v.is_null()) {
            append(&mut self.text, text, TOTAL_LIMIT)?;
            if self.reasoning.is_empty() {
                self.emitter.text(text.as_str().unwrap(), &mut out)?;
            }
        }
        if let Some(calls) = delta.get("tool_calls") {
            for call in calls.as_array().ok_or("OpenAI tools invalid")? {
                let index = call["index"]
                    .as_u64()
                    .filter(|i| *i < 128)
                    .ok_or("OpenAI tool index invalid")?;
                let tool = self.tools.entry(index).or_default();
                if let Some(id) = call.get("id") {
                    let id = id.as_str().ok_or("OpenAI tool id invalid")?;
                    if !tool.id.is_empty() && tool.id != id {
                        return Err("OpenAI tool id changed");
                    }
                    tool.id = id.to_string();
                }
                if call
                    .get("type")
                    .is_some_and(|v| v.as_str() != Some("function"))
                {
                    return Err("OpenAI tool type unsupported");
                }
                if let Some(function) = call.get("function") {
                    if let Some(name) = function.get("name") {
                        append(&mut tool.name, name, 256)?;
                    }
                    if let Some(args) = function.get("arguments") {
                        append(&mut tool.args, args, FRAME_LIMIT)?;
                    }
                }
            }
        }
        if let Some(reason) = choice.get("finish_reason").filter(|v| !v.is_null()) {
            self.finish = Some(reason.as_str().ok_or("OpenAI finish invalid")?.to_string());
        }
        Ok(out)
    }
}
pub(crate) struct Responses<'a> {
    emitter: Emitter,
    id: String,
    text: String,
    items: BTreeMap<u64, Value>,
    arguments: BTreeMap<u64, String>,
    texts: BTreeMap<(u64, u64), String>,
    reasoning: BTreeMap<(u64, bool, u64), String>,
    model: &'a str,
    target: &'a str,
    signer: &'a openai_chat::ReasoningSigner,
    known: &'a Map<String, Value>,
    sequence: Option<u64>,
}
impl<'a> Responses<'a> {
    pub(crate) fn new(
        model: &'a str,
        target: &'a str,
        signer: &'a openai_chat::ReasoningSigner,
        known: &'a Map<String, Value>,
    ) -> Self {
        Self {
            emitter: Emitter::default(),
            id: String::new(),
            text: String::new(),
            items: BTreeMap::new(),
            arguments: BTreeMap::new(),
            texts: BTreeMap::new(),
            reasoning: BTreeMap::new(),
            model,
            target,
            signer,
            known,
            sequence: None,
        }
    }
    pub(crate) fn complete(&self) -> bool {
        self.emitter.complete
    }
    pub(crate) fn apply(&mut self, frame: Frame) -> Result<Vec<Event>> {
        if self.complete() {
            return Err("Responses data follows completion");
        }
        let Frame::Json(value) = frame else {
            return Err("Responses ended without completion");
        };
        let mut out = Vec::new();
        if let Some(seq) = value.get("sequence_number") {
            let seq = seq.as_u64().ok_or("invalid sequence")?;
            if self.sequence.is_some_and(|old| seq <= old) {
                return Err("Responses sequence regressed");
            }
            self.sequence = Some(seq);
        }
        let kind = value["type"]
            .as_str()
            .ok_or("Responses event type missing")?;
        if kind == "response.created" {
            let id = value["response"]["id"]
                .as_str()
                .ok_or("Responses id missing")?;
            self.id = id.to_string();
            self.emitter.start(id, self.model, &mut out)?;
            return Ok(out);
        }
        if !self.emitter.started {
            return Err("Responses data before created");
        }
        match kind {
            "response.in_progress" => {
                if value["response"]["id"].as_str() != Some(self.id.as_str()) {
                    return Err("Responses id changed");
                }
            }
            "response.output_item.added" => {
                let index = value["output_index"]
                    .as_u64()
                    .filter(|i| *i < 128)
                    .ok_or("Responses item index invalid")?;
                let item = &value["item"];
                if self.items.contains_key(&index)
                    || !item.is_object()
                    || item["id"].as_str().is_none()
                {
                    return Err("Responses item invalid");
                }
                if !matches!(
                    item["type"].as_str(),
                    Some("message" | "function_call" | "reasoning")
                ) {
                    return Err("Responses output type unsupported");
                }
                if item["type"] == "function_call" {
                    self.arguments
                        .insert(index, item["arguments"].as_str().unwrap_or("").to_string());
                }
                self.items.insert(index, item.clone());
            }
            "response.output_text.delta" => {
                let index = self.validate_item(&value, "message")?;
                let content_index = value["content_index"]
                    .as_u64()
                    .filter(|i| *i < 128)
                    .ok_or("Responses content index invalid")?;
                append(
                    self.texts.entry((index, content_index)).or_default(),
                    &value["delta"],
                    TOTAL_LIMIT,
                )?;
                append(&mut self.text, &value["delta"], TOTAL_LIMIT)?;
                // Anthropic blocks are sequential. Hold text behind earlier
                // reasoning/tools so completing them cannot reorder content.
                if self.items.range(..index).count() == index as usize
                    && !self
                        .items
                        .range(..index)
                        .any(|(_, item)| item["type"] != "message")
                {
                    self.emitter
                        .text(value["delta"].as_str().unwrap(), &mut out)?;
                }
            }
            "response.function_call_arguments.delta" => {
                let index = self.validate_item(&value, "function_call")?;
                append(
                    self.arguments
                        .get_mut(&index)
                        .ok_or("missing function item")?,
                    &value["delta"],
                    FRAME_LIMIT,
                )?;
            }
            "response.function_call_arguments.done" => {
                let index = self.validate_item(&value, "function_call")?;
                if self.arguments.get(&index).map(String::as_str) != value["arguments"].as_str() {
                    return Err("Responses tool snapshot changed");
                }
            }
            "response.output_item.done" => {
                let index = value["output_index"].as_u64().ok_or("invalid item index")?;
                let item = self.items.get(&index).ok_or("missing output item")?;
                if item["id"] != value["item"]["id"] || item["type"] != value["item"]["type"] {
                    return Err("Responses item identity changed");
                }
                if item["type"] == "function_call"
                    && (item["name"] != value["item"]["name"]
                        || item["call_id"] != value["item"]["call_id"]
                        || self.arguments.get(&index).map(String::as_str)
                            != value["item"]["arguments"].as_str())
                {
                    return Err("Responses tool snapshot changed");
                }
            }
            "response.completed" | "response.incomplete" => {
                let response = &value["response"];
                if response["id"].as_str() != Some(self.id.as_str())
                    || response["status"].as_str()
                        != Some(if kind == "response.completed" {
                            "completed"
                        } else {
                            "incomplete"
                        })
                {
                    return Err("Responses completion invalid");
                }
                if kind == "response.incomplete"
                    && response["incomplete_details"]["reason"].as_str()
                        != Some("max_output_tokens")
                {
                    return Err("Responses incomplete reason unsupported");
                }
                let output = response["output"]
                    .as_array()
                    .ok_or("Responses output missing")?;
                if output.len() != self.items.len() {
                    return Err("Responses output snapshot mismatch");
                }
                for (index, item) in output.iter().enumerate() {
                    let prior = self
                        .items
                        .get(&(index as u64))
                        .ok_or("Responses indices sparse")?;
                    if prior["id"] != item["id"] || prior["type"] != item["type"] {
                        return Err("Responses output identity changed");
                    }
                    if item["type"] == "function_call"
                        && (prior["name"] != item["name"]
                            || prior["call_id"] != item["call_id"]
                            || self.arguments.get(&(index as u64)).map(String::as_str)
                                != item["arguments"].as_str())
                    {
                        return Err("Responses tool snapshot mismatch");
                    }
                }
                for (index, item) in output.iter().enumerate() {
                    if item["type"] == "message" {
                        let content = item["content"]
                            .as_array()
                            .ok_or("Responses content snapshot invalid")?;
                        for (ci, part) in content.iter().enumerate() {
                            if self
                                .texts
                                .get(&(index as u64, ci as u64))
                                .map(String::as_str)
                                .unwrap_or("")
                                != part["text"].as_str().unwrap_or("")
                            {
                                return Err("Responses content snapshot mismatch");
                            }
                        }
                        if self
                            .texts
                            .keys()
                            .any(|(i, c)| *i == index as u64 && *c >= content.len() as u64)
                        {
                            return Err("Responses content snapshot missing");
                        }
                    }
                }
                for ((index, summary, part), text) in &self.reasoning {
                    let field = if *summary { "summary" } else { "content" };
                    if output
                        .get(*index as usize)
                        .and_then(|i| i[field].get(*part as usize))
                        .and_then(|p| p["text"].as_str())
                        != Some(text.as_str())
                    {
                        return Err("Responses reasoning snapshot mismatch");
                    }
                }
                let mapped = openai_responses::openai_to_anthropic_with_reasoning(
                    response,
                    self.model,
                    self.target,
                    self.known,
                    self.signer,
                )
                .map_err(|_| "Responses completion validation failed")?;
                let text: String = mapped["content"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|b| b["type"] == "text")
                    .filter_map(|b| b["text"].as_str())
                    .collect();
                if text != self.text {
                    return Err("Responses text snapshot mismatch");
                }
                self.emitter.finish(&mapped, &mut out)?;
            }
            "response.output_text.done" => {
                let index = self.validate_item(&value, "message")?;
                let ci = value["content_index"]
                    .as_u64()
                    .ok_or("Responses content index invalid")?;
                if self
                    .texts
                    .get(&(index, ci))
                    .map(String::as_str)
                    .unwrap_or("")
                    != value["text"]
                        .as_str()
                        .ok_or("Responses text snapshot missing")?
                {
                    return Err("Responses text done mismatch");
                }
            }
            "response.content_part.added" | "response.content_part.done" => {
                let index = self.validate_item(&value, "message")?;
                let ci = value["content_index"]
                    .as_u64()
                    .filter(|i| *i < 128)
                    .ok_or("Responses content index invalid")?;
                let part = &value["part"];
                if part["type"] != "output_text" || !part["text"].is_string() {
                    return Err("Responses output media or refusal is unsupported");
                }
                if kind.ends_with(".done")
                    && self
                        .texts
                        .get(&(index, ci))
                        .map(String::as_str)
                        .unwrap_or("")
                        != part["text"].as_str().unwrap()
                {
                    return Err("Responses content part mismatch");
                }
                if kind.ends_with(".added") && part["text"] != "" {
                    return Err("Responses initial content must be empty");
                }
            }
            "response.reasoning_summary_text.delta"
            | "response.reasoning_text.delta"
            | "response.reasoning_summary_text.done"
            | "response.reasoning_text.done" => {
                let index = self.validate_item(&value, "reasoning")?;
                let summary = kind.starts_with("response.reasoning_summary_");
                let pi = value[if summary {
                    "summary_index"
                } else {
                    "content_index"
                }]
                .as_u64()
                .filter(|i| *i < 128)
                .ok_or("Responses reasoning index invalid")?;
                let text = self.reasoning.entry((index, summary, pi)).or_default();
                if kind.ends_with(".delta") {
                    append(text, &value["delta"], FRAME_LIMIT)?;
                } else if Some(text.as_str()) != value["text"].as_str() {
                    return Err("Responses reasoning done mismatch");
                }
            }
            "response.reasoning_summary_part.added" | "response.reasoning_summary_part.done" => {
                let index = self.validate_item(&value, "reasoning")?;
                let pi = value["summary_index"]
                    .as_u64()
                    .filter(|i| *i < 128)
                    .ok_or("Responses summary index invalid")?;
                if value["part"]["type"] != "summary_text" || !value["part"]["text"].is_string() {
                    return Err("Responses summary part invalid");
                }
                if kind.ends_with(".done")
                    && self
                        .reasoning
                        .get(&(index, true, pi))
                        .map(String::as_str)
                        .unwrap_or("")
                        != value["part"]["text"].as_str().unwrap()
                {
                    return Err("Responses summary snapshot mismatch");
                }
                if kind.ends_with(".added") && value["part"]["text"] != "" {
                    return Err("Responses initial summary must be empty");
                }
            }
            _ => return Err("Responses event unsupported or upstream failed"),
        }
        Ok(out)
    }
    fn validate_item(&self, value: &Value, kind: &str) -> Result<u64> {
        let index = value["output_index"]
            .as_u64()
            .ok_or("Responses index missing")?;
        let item = self.items.get(&index).ok_or("Responses item not created")?;
        if item["type"].as_str() != Some(kind) || item["id"] != value["item_id"] {
            return Err("Responses item identity mismatch");
        }
        Ok(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decoder_split_utf8_crlf_multiline_bounded() {
        let bytes =
            "event: x\r\ndata: {\"type\":\"x\",\r\ndata: \"delta\":\"你好\"}\r\n\r\n".as_bytes();
        let mut decoder = Decoder::default();
        let mut values = Vec::new();
        for byte in bytes {
            values.extend(decoder.feed(&[*byte]).unwrap());
        }
        assert!(matches!(&values[0], Frame::Json(v) if v["delta"] == "你好"));
        assert!(Decoder::default()
            .feed(&vec![b'x'; FRAME_LIMIT + 1])
            .is_err());
    }
    #[test]
    fn chat_text_precedes_finish_and_tool_fragments_are_validated() {
        let signer = openai_chat::ReasoningSigner::new("fixture", "fixture").unwrap();
        let known = Map::from_iter([(
            "echo".to_string(),
            json!({"type":"object","required":["x"],"properties":{"x":{"type":"string"}}}),
        )]);
        let mut chat = Chat::new("alias", "fixture", &signer, &known);
        let chunk = |delta, finish| {
            Frame::Json(
                json!({"id":"c","choices":[{"index":0,"delta":delta,"finish_reason":finish}]}),
            )
        };
        assert!(chat
            .apply(chunk(json!({"content":"first"}), Value::Null))
            .unwrap()
            .iter()
            .any(|(_, v)| v["delta"]["text"] == "first"));
        chat.apply(chunk(json!({"tool_calls":[{"index":0,"id":"call1","type":"function","function":{"name":"echo","arguments":"{\"x\":"}}]}), Value::Null)).unwrap();
        chat.apply(chunk(
            json!({"tool_calls":[{"index":0,"function":{"arguments":"\"ok\"}"}}]}),
            json!("tool_calls"),
        ))
        .unwrap();
        let events = chat.apply(Frame::Done).unwrap();
        assert_eq!(
            events.iter().filter(|(e, _)| e == "message_stop").count(),
            1
        );
        assert!(chat.apply(Frame::Done).is_err());
    }
    #[test]
    fn chat_malformed_json_and_missing_finish_never_succeed() {
        let signer = openai_chat::ReasoningSigner::new("fixture", "fixture").unwrap();
        let known = Map::new();
        let mut chat = Chat::new("a", "b", &signer, &known);
        chat.apply(Frame::Json(json!({"id":"c","choices":[{"index":0,"delta":{"content":"partial"},"finish_reason":null}]}))).unwrap();
        assert!(chat.apply(Frame::Done).is_err());
        assert!(!chat.complete());
    }
    fn response_frame(value: Value) -> Frame {
        Frame::Json(value)
    }

    #[test]
    fn responses_reasoning_summary_is_verified_signed_and_released_at_completion() {
        let signer = openai_chat::ReasoningSigner::new("fixture", "profile").unwrap();
        let known = Map::new();
        let mut stream = Responses::new("alias", "fixture-model", &signer, &known);
        stream
            .apply(response_frame(
                json!({"type":"response.created","response":{"id":"resp"}}),
            ))
            .unwrap();
        stream.apply(response_frame(json!({"type":"response.output_item.added","output_index":0,"item":{"type":"reasoning","id":"r1","summary":[]}}))).unwrap();
        let early = stream.apply(response_frame(json!({"type":"response.reasoning_summary_text.delta","output_index":0,"item_id":"r1","summary_index":0,"delta":"fixture summary"}))).unwrap();
        assert!(early.is_empty());
        stream.apply(response_frame(json!({"type":"response.reasoning_summary_text.done","output_index":0,"item_id":"r1","summary_index":0,"text":"fixture summary"}))).unwrap();
        let output = stream.apply(response_frame(json!({"type":"response.completed","response":{"id":"resp","status":"completed","output":[{"type":"reasoning","id":"r1","summary":[{"type":"summary_text","text":"fixture summary"}],"encrypted_content":"opaque-fixture"}]}}))).unwrap();
        assert!(output
            .iter()
            .any(|(_, v)| v["delta"]["thinking"] == "fixture summary"));
        assert!(output.iter().any(|(_, v)| v["delta"]["signature"]
            .as_str()
            .is_some_and(|s| s.starts_with("csswitch.openai-responses-thinking.v1."))));
        assert_eq!(
            output
                .iter()
                .filter(|(event, _)| event == "message_stop")
                .count(),
            1
        );
    }

    #[test]
    fn responses_changed_reasoning_snapshot_and_wrong_item_identity_fail() {
        let signer = openai_chat::ReasoningSigner::new("fixture", "profile").unwrap();
        let known = Map::new();
        let mut stream = Responses::new("alias", "fixture-model", &signer, &known);
        stream
            .apply(response_frame(
                json!({"type":"response.created","response":{"id":"resp"}}),
            ))
            .unwrap();
        stream.apply(response_frame(json!({"type":"response.output_item.added","output_index":0,"item":{"type":"reasoning","id":"r1","summary":[]}}))).unwrap();
        assert!(stream.apply(response_frame(json!({"type":"response.reasoning_summary_text.delta","output_index":0,"item_id":"foreign","summary_index":0,"delta":"hidden"}))).is_err());
        stream.apply(response_frame(json!({"type":"response.reasoning_summary_text.delta","output_index":0,"item_id":"r1","summary_index":0,"delta":"first"}))).unwrap();
        assert!(stream.apply(response_frame(json!({"type":"response.completed","response":{"id":"resp","status":"completed","output":[{"type":"reasoning","id":"r1","summary":[{"type":"summary_text","text":"different"}]}]}}))).is_err());
        assert!(!stream.complete());
    }

    #[test]
    fn responses_content_boundaries_cannot_be_changed_by_same_concatenated_text() {
        let signer = openai_chat::ReasoningSigner::new("fixture", "profile").unwrap();
        let known = Map::new();
        let mut stream = Responses::new("alias", "fixture-model", &signer, &known);
        stream
            .apply(response_frame(
                json!({"type":"response.created","response":{"id":"resp"}}),
            ))
            .unwrap();
        stream.apply(response_frame(json!({"type":"response.output_item.added","output_index":0,"item":{"type":"message","id":"m1","content":[]}}))).unwrap();
        for (i, text) in [(0, "a"), (1, "bc")] {
            stream.apply(response_frame(json!({"type":"response.output_text.delta","output_index":0,"item_id":"m1","content_index":i,"delta":text}))).unwrap();
        }
        assert!(stream.apply(response_frame(json!({"type":"response.completed","response":{"id":"resp","status":"completed","output":[{"type":"message","id":"m1","content":[{"type":"output_text","text":"ab"},{"type":"output_text","text":"c"}]}]}}))).is_err());
        assert!(!stream.complete());
    }

    #[test]
    fn responses_output_media_and_partial_eof_never_complete_as_text_only_success() {
        let signer = openai_chat::ReasoningSigner::new("fixture", "profile").unwrap();
        let known = Map::new();
        let mut stream = Responses::new("alias", "fixture-model", &signer, &known);
        stream
            .apply(response_frame(
                json!({"type":"response.created","response":{"id":"resp"}}),
            ))
            .unwrap();
        stream.apply(response_frame(json!({"type":"response.output_item.added","output_index":0,"item":{"type":"message","id":"m1","content":[]}}))).unwrap();
        assert!(stream.apply(response_frame(json!({"type":"response.content_part.added","output_index":0,"item_id":"m1","content_index":0,"part":{"type":"output_audio","data":"fixture"}}))).is_err());
        assert!(stream.apply(Frame::Done).is_err());
        assert!(!stream.complete());
    }

    #[test]
    fn responses_opaque_reasoning_emits_redacted_block_without_fabricated_text() {
        let signer = openai_chat::ReasoningSigner::new("fixture", "profile").unwrap();
        let known = Map::new();
        let mut stream = Responses::new("alias", "fixture-model", &signer, &known);
        stream
            .apply(response_frame(
                json!({"type":"response.created","response":{"id":"resp"}}),
            ))
            .unwrap();
        stream.apply(response_frame(json!({"type":"response.output_item.added","output_index":0,"item":{"type":"reasoning","id":"r1","summary":[]}}))).unwrap();
        let output = stream.apply(response_frame(json!({"type":"response.completed","response":{"id":"resp","status":"completed","output":[{"type":"reasoning","id":"r1","summary":[],"encrypted_content":"opaque-fixture"}]}}))).unwrap();
        assert!(output
            .iter()
            .any(|(_, v)| v["content_block"]["type"] == "redacted_thinking"));
        assert!(!output
            .iter()
            .any(|(_, v)| v["delta"]["type"] == "thinking_delta"));
    }

    #[test]
    fn chat_reasoning_precedes_text_and_is_not_reordered_at_completion() {
        let signer = openai_chat::ReasoningSigner::new("fixture", "profile").unwrap();
        let known = Map::new();
        let mut stream = Chat::new("alias", "model", &signer, &known);
        let early = stream.apply(Frame::Json(json!({"id":"chat","choices":[{"index":0,"delta":{"reasoning_content":"summary","content":"answer"},"finish_reason":"stop"}]}))).unwrap();
        assert!(!early
            .iter()
            .any(|(_, v)| v["delta"]["type"] == "text_delta"));
        let events = stream.apply(Frame::Done).unwrap();
        let kinds: Vec<_> = events
            .iter()
            .filter(|(_, v)| v["type"] == "content_block_start")
            .map(|(_, v)| v["content_block"]["type"].as_str().unwrap())
            .collect();
        assert_eq!(kinds, vec!["thinking", "text"]);
    }

    #[test]
    fn decoder_requires_a_clean_frame_boundary_at_eof() {
        let mut complete = Decoder::default();
        complete.feed(b"data: [DONE]\n\n").unwrap();
        assert!(complete.finish().is_ok());
        let mut partial = Decoder::default();
        partial.feed(b"data: [DONE]\n\ndata: {\"error\":").unwrap();
        assert!(partial.finish().is_err());
    }
}
