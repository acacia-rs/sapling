use super::ParseError;

/// One line of a command's answer. `text` is shown as written, or, when it is one of the client's
/// translation keys, in the client's language with `parameters` filled in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputMessage {
    pub text: String,
    pub parameters: Vec<String>,
    pub success: bool,
}

/// What a command tells its sender, sent when it returns.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Output {
    messages: Vec<OutputMessage>,
    echoed: bool,
    done_to: Option<u32>,
    always: bool,
}

impl Output {
    /// The lines other operators are shown, as BDS echoes the success of an operator's command to them.
    pub fn echoed(&self) -> impl Iterator<Item = &OutputMessage> {
        self.messages.iter().filter(|m| self.echoed && m.success)
    }

    pub(super) fn echo(&mut self) {
        self.echoed = true;
    }

    pub fn print(&mut self, text: impl Into<String>) {
        self.translate(text, Vec::new(), true);
    }

    /// A line the client shows in red; the command counts as failed.
    pub fn error(&mut self, text: impl Into<String>) {
        self.translate(text, Vec::new(), false);
    }

    /// A line under one of the client's translation keys (`commands.generic.unknown`).
    pub fn translate(&mut self, key: impl Into<String>, parameters: Vec<String>, success: bool) {
        self.messages.push(OutputMessage { text: key.into(), parameters, success });
    }

    pub fn messages(&self) -> &[OutputMessage] {
        &self.messages
    }

    /// Whether the command did what it was asked: no line is an error.
    pub fn succeeded(&self) -> bool {
        self.messages.iter().all(|m| m.success)
    }

    /// Says the command acted on `count` targets, which its sender is told (BDS's `successCount`).
    pub fn done_to(&mut self, count: u32) {
        self.done_to = Some(count);
    }

    /// Has the sender answered even where the world's `sendCommandFeedback` is off, as `/tell` is.
    pub fn always_answer(&mut self) {
        self.always = true;
    }

    /// Whether the answer goes to the sender whatever `sendCommandFeedback` says.
    pub fn answers_always(&self) -> bool {
        self.always
    }

    /// How many targets the command acted on: 0 if no line succeeded, 1 unless it said
    /// ([`Output::done_to`]). `/clear @a` clearing one player of two counted 1
    /// (research/captures/bds-itemcmds.txt).
    pub fn success_count(&self) -> u32 {
        if self.messages.iter().any(|m| m.success) { self.done_to.unwrap_or(1) } else { 0 }
    }

    /// The answer to a line that was not run, under the keys BDS uses; for a command whose
    /// arguments are only found wrong once it runs.
    pub fn refuse(&mut self, error: ParseError) {
        match error {
            ParseError::Syntax { before, token, after } => self.translate("commands.generic.syntax", vec![before, token, after], false),
            ParseError::NoTarget => self.translate("commands.generic.noTargetMatch", Vec::new(), false),
            ParseError::Invalid(key) => self.translate(key, Vec::new(), false),
        }
    }
}
