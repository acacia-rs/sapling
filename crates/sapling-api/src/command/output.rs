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
}

impl Output {
    /// The lines other operators are shown, as BDS echoes an operator command's success to them.
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

    /// The answer to a line that was not run, under the keys BDS uses.
    pub(super) fn refuse(&mut self, error: ParseError) {
        match error {
            ParseError::Syntax { before, token, after } => self.translate("commands.generic.syntax", vec![before, token, after], false),
            ParseError::NoTarget => self.translate("commands.generic.noTargetMatch", Vec::new(), false),
        }
    }
}
