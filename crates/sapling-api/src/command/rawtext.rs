//! The client's own text as `/tellraw` and `/titleraw` take it: `{"rawtext":[...]}` of text,
//! translations, selectors and scores, checked with BDS's errors (research/captures/bds-messaging.txt).

use serde_json::{Map, Value, json};

use super::{Args, ParamKind, Parameter, ParseError};

/// A line of rawtext. Selectors and scores stand as written until [`RawText::resolve`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RawText(pub Vec<RawPart>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawPart {
    Text(String),
    /// A translation key, `%s` filled from `with`.
    Translate { key: String, with: Option<RawText> },
    /// The names of who a selector (`@a`, a name) finds, joined by `, `.
    Selector(String),
    /// What `name` (`@s`, a name, `*` for whoever reads it) scores on `objective`.
    Score { name: String, objective: String },
}

const ERROR: &str = "commands.tellraw.error.";

fn error(what: &str) -> String {
    format!("{ERROR}{what}")
}

fn string(part: &Map<String, Value>, key: &str, what: &str) -> Result<String, String> {
    part.get(key).and_then(Value::as_str).map(str::to_owned).ok_or_else(|| error(what))
}

impl RawText {
    /// Reads `{"rawtext":[...]}`; `Err` is the key BDS answers with. A part with none of the
    /// keys BDS knows is left out, as BDS left `{"bogus":"x"}` out.
    pub fn from_json(value: &Value) -> Result<Self, String> {
        let parts = value.get("rawtext").and_then(Value::as_array).ok_or_else(|| error("notArray"))?;
        let mut read = Vec::new();
        for part in parts {
            let part = part.as_object().ok_or_else(|| error("itemIsNotObject"))?;
            // TODO: which key wins in a part with several was not drilled.
            read.push(if part.contains_key("text") {
                RawPart::Text(string(part, "text", "textNotString")?)
            } else if part.contains_key("selector") {
                RawPart::Selector(string(part, "selector", "selectorNotString")?)
            } else if part.contains_key("translate") {
                RawPart::Translate { key: string(part, "translate", "translateNotString")?, with: part.get("with").map(Self::with).transpose()? }
            } else if let Some(score) = part.get("score") {
                let score = score.as_object().ok_or_else(|| error("scoreNotObject"))?;
                RawPart::Score { name: string(score, "name", "scoreNameNotString")?, objective: string(score, "objective", "scoreObjectiveNotString")? }
            } else {
                continue;
            });
        }
        Ok(Self(read))
    }

    /// A translation's `with`: an array of strings or a rawtext of its own.
    fn with(value: &Value) -> Result<Self, String> {
        let not = || error("withNotArrayOrRawText");
        match value {
            Value::Array(words) => words.iter().map(|word| word.as_str().map(|word| RawPart::Text(word.to_owned()))).collect::<Option<_>>().map(Self).ok_or_else(not),
            Value::Object(_) => Self::from_json(value).map_err(|key| if key == error("notArray") { not() } else { key }),
            _ => Err(not()),
        }
    }

    /// The text with each selector and score, in translations too, replaced by what `resolve`
    /// makes of it; `None` leaves it out.
    pub fn resolve(&self, resolve: &mut impl FnMut(&RawPart) -> Option<RawPart>) -> Self {
        let resolved = self.0.iter().filter_map(|part| match part {
            RawPart::Text(_) => Some(part.clone()),
            RawPart::Translate { key, with } => Some(RawPart::Translate { key: key.clone(), with: with.as_ref().map(|with| with.resolve(resolve)) }),
            RawPart::Selector(_) | RawPart::Score { .. } => resolve(part),
        });
        Self(resolved.collect())
    }

    /// As BDS sends it: `{"rawtext":[...]}` and a newline; no parts is `null`, `with` is always a rawtext.
    pub fn to_json(&self) -> String {
        format!("{}\n", self.value())
    }

    fn value(&self) -> Value {
        let parts: Vec<Value> = self.0.iter().map(RawPart::value).collect();
        json!({ "rawtext": if parts.is_empty() { Value::Null } else { Value::Array(parts) } })
    }
}

impl RawPart {
    fn value(&self) -> Value {
        match self {
            Self::Text(text) => json!({ "text": text }),
            Self::Translate { key, with: None } => json!({ "translate": key }),
            Self::Translate { key, with: Some(with) } => json!({ "translate": key, "with": with.value() }),
            Self::Selector(selector) => json!({ "selector": selector }),
            Self::Score { name, objective } => json!({ "score": { "name": name, "objective": objective } }),
        }
    }
}

impl Parameter for RawText {
    fn kind() -> ParamKind {
        ParamKind::Json
    }

    /// Text that is no JSON object is a syntax error at its first word, as on BDS.
    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let mark = args.mark();
        let value: Value = serde_json::from_str(args.json()?).map_err(|_| args.error_at(mark))?;
        Self::from_json(&value).map_err(ParseError::Invalid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Sender;

    fn parsed(line: &str) -> Result<RawText, ParseError> {
        let sender = Sender::Console;
        let mut args = Args::new(line, 0, &sender, &[]);
        RawText::parse(&mut args).and_then(|text| args.finish().map(|()| text))
    }

    fn invalid(what: &str) -> Result<RawText, ParseError> {
        Err(ParseError::Invalid(error(what)))
    }

    #[test]
    fn parts_read_as_bds_reads_them() {
        let text = parsed(r#"{"rawtext":[{"text":"hi "},{"translate":"a.b","with":["x","y"]},{"selector":"@a"},{"score":{"name":"*","objective":"obj"}},{"bogus":1}]}"#).unwrap();
        let with = RawText(vec![RawPart::Text("x".into()), RawPart::Text("y".into())]);
        let score = RawPart::Score { name: "*".into(), objective: "obj".into() };
        assert_eq!(text.0, [RawPart::Text("hi ".into()), RawPart::Translate { key: "a.b".into(), with: Some(with) }, RawPart::Selector("@a".into()), score]);
        let nested = parsed(r#"{"rawtext":[{"translate":"%%s","with":{"rawtext":[{"selector":"@s"}]}}]}"#).unwrap();
        assert_eq!(nested.0, [RawPart::Translate { key: "%%s".into(), with: Some(RawText(vec![RawPart::Selector("@s".into())])) }]);
        assert_eq!(parsed(r#"{ "rawtext" : [ { "text" : "a ] }" } ] }"#).unwrap().0, [RawPart::Text("a ] }".into())], "spaces, and brackets in strings");
        assert_eq!(parsed(r#"{"rawtext":[{"text":"\"q\\"}]}"#).unwrap().0, [RawPart::Text("\"q\\".into())]);
    }

    #[test]
    fn what_bds_refuses_gets_its_key() {
        assert_eq!(parsed(r#"{"text":"x"}"#), invalid("notArray"));
        assert_eq!(parsed(r#"{"rawtext":"x"}"#), invalid("notArray"));
        assert_eq!(parsed(r#"{"rawtext":[5]}"#), invalid("itemIsNotObject"));
        assert_eq!(parsed(r#"{"rawtext":[{"text":5}]}"#), invalid("textNotString"));
        assert_eq!(parsed(r#"{"rawtext":[{"selector":[]}]}"#), invalid("selectorNotString"));
        assert_eq!(parsed(r#"{"rawtext":[{"translate":1}]}"#), invalid("translateNotString"));
        assert_eq!(parsed(r#"{"rawtext":[{"translate":"x","with":5}]}"#), invalid("withNotArrayOrRawText"));
        assert_eq!(parsed(r#"{"rawtext":[{"translate":"x","with":{"text":"y"}}]}"#), invalid("withNotArrayOrRawText"));
        assert_eq!(parsed(r#"{"rawtext":[{"score":"x"}]}"#), invalid("scoreNotObject"));
        assert_eq!(parsed(r#"{"rawtext":[{"score":{"objective":"o"}}]}"#), invalid("scoreNameNotString"));
        assert_eq!(parsed(r#"{"rawtext":[{"score":{"name":"@s"}}]}"#), invalid("scoreObjectiveNotString"));
        let syntax = |before: &str, token: &str, after: &str| Err(ParseError::Syntax { before: before.into(), token: token.into(), after: after.into() });
        assert_eq!(parsed("not json"), syntax("/", "not", " json"));
        assert_eq!(parsed(r#"{"rawtext":[{"text":"a"}]} extra"#), syntax(r#"t":"a"}]} "#, "extra", ""), "the line BDS answered");
        assert_eq!(parsed(r#"{"rawtext":[{"text":"a"}]"#), syntax("/", r#"{"rawtext":[{"text":"a"}]"#, ""), "unclosed");
        assert_eq!(parsed(r#"{rawtext}"#), syntax("/", "{rawtext}", ""));
    }

    #[test]
    fn resolved_text_is_written_as_bds_sends_it() {
        let text = parsed(r#"{"rawtext":[{"text":"hi "},{"translate":"k","with":{"rawtext":[{"selector":"@s"},{"score":{"name":"@s","objective":"o"}}]}},{"selector":"@e"},{"translate":"plain"}]}"#).unwrap();
        let resolved = text.resolve(&mut |part| match part {
            RawPart::Selector(selector) if selector == "@s" => Some(RawPart::Text("Alex".into())),
            _ => None,
        });
        assert_eq!(resolved.to_json(), "{\"rawtext\":[{\"text\":\"hi \"},{\"translate\":\"k\",\"with\":{\"rawtext\":[{\"text\":\"Alex\"}]}},{\"translate\":\"plain\"}]}\n");
        assert_eq!(RawText::default().to_json(), "{\"rawtext\":null}\n", "BDS's answer to an empty one");
        let score = RawText(vec![RawPart::Score { name: "*".into(), objective: "o".into() }]);
        assert_eq!(score.to_json(), "{\"rawtext\":[{\"score\":{\"name\":\"*\",\"objective\":\"o\"}}]}\n");
    }
}
