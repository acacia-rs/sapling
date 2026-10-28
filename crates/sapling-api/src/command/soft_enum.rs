//! Soft enums: the words a command parameter is completed from, changed while the server runs
//! (UpdateSoftEnum to every player of the shard), as BDS's `CommandRegistry` soft enums. A soft
//! enum parameter takes any word; the command checks it against [`ShardCtx::soft_enum`].
//!
//! ```ignore
//! #[derive(Command)]
//! #[command(name = "join")]
//! struct Join {
//!     #[command(soft_enum = "Arena")]
//!     arena: String,
//! }
//! // When an arena opens:
//! ctx.add_soft_enum_values("Arena", &["castle".into()]);
//! ```

use crate::ShardCtx;

/// What a change does to a soft enum's values, as UpdateSoftEnum's actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SoftEnumChange {
    /// Adds the words it lacks.
    Add(Vec<String>),
    /// Takes the words away.
    Remove(Vec<String>),
    /// Replaces every word.
    Set(Vec<String>),
}

impl SoftEnumChange {
    /// `values` after the change; a word is kept once.
    pub fn apply(&self, values: &mut Vec<String>) {
        match self {
            Self::Add(words) => words.iter().for_each(|word| {
                if !values.contains(word) {
                    values.push(word.clone());
                }
            }),
            Self::Remove(words) => values.retain(|value| !words.contains(value)),
            Self::Set(words) => {
                values.clear();
                Self::Add(words.clone()).apply(values);
            }
        }
    }
}

impl ShardCtx<'_> {
    /// The words the soft enum `name` has now on this shard: its parameters' first values until a
    /// change.
    pub fn soft_enum(&self, name: &str) -> Vec<String> {
        self.backend.soft_enum(name)
    }

    /// Replaces the soft enum's words, for every player of the shard and those who come later.
    pub fn set_soft_enum(&mut self, name: &str, values: &[String]) {
        self.backend.change_soft_enum(name, SoftEnumChange::Set(values.to_vec()));
    }

    pub fn add_soft_enum_values(&mut self, name: &str, values: &[String]) {
        self.backend.change_soft_enum(name, SoftEnumChange::Add(values.to_vec()));
    }

    pub fn remove_soft_enum_values(&mut self, name: &str, values: &[String]) {
        self.backend.change_soft_enum(name, SoftEnumChange::Remove(values.to_vec()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changes_keep_each_word_once() {
        let words = |list: &[&str]| list.iter().map(|word| (*word).to_owned()).collect::<Vec<_>>();
        let mut values = words(&["a"]);
        SoftEnumChange::Add(words(&["b", "a", "c"])).apply(&mut values);
        assert_eq!(values, words(&["a", "b", "c"]));
        SoftEnumChange::Remove(words(&["a", "z"])).apply(&mut values);
        assert_eq!(values, words(&["b", "c"]));
        SoftEnumChange::Set(words(&["x", "x"])).apply(&mut values);
        assert_eq!(values, words(&["x"]));
    }
}
