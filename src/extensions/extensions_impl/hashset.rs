use std::collections::HashSet;
use crate::extensions::JoinString;

impl JoinString for HashSet<String> {
    fn join(&self, separator: &str) -> String {
        self.iter().cloned().collect::<Vec<_>>().join(separator)
    }
}
impl JoinString for HashSet<&str> {
    fn join(&self, separator: &str) -> String {
        self.iter().cloned().collect::<Vec<_>>().join(separator)
    }
}
