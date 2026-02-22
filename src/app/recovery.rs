#[must_use] 
pub fn get_suggestions(msg: &str) -> Vec<String> {
    let mut suggestions = Vec::new();
    let msg_lower = msg.to_lowercase();

    if msg_lower.contains("dirty working copy") || msg_lower.contains("uncommitted changes") {
        suggestions.push("Try running: jj snapshot".to_string());
    }

    if msg_lower.contains("immutable")
        && (msg_lower.contains("edit") || msg_lower.contains("describe"))
    {
        suggestions
            .push("Try running: jj new (to create a child of the immutable revision)".to_string());
    }

    if msg_lower.contains("conflict") {
        suggestions.push("Try running: jj resolve (to open the external merge tool)".to_string());
    }

    if msg_lower.contains("no such bookmark") {
        suggestions.push("Check the bookmark name or try: jj bookmark list".to_string());
    }

    if msg_lower.contains("not a git repository") {
        suggestions.push("Ensure you are in a jj/git repository or try: jj git init".to_string());
    }

    suggestions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggestions() {
        let s = get_suggestions("error: Dirty working copy");
        assert!(s.contains(&"Try running: jj snapshot".to_string()));

        let s = get_suggestions("error: Cannot edit immutable revision");
        assert!(s.contains(
            &"Try running: jj new (to create a child of the immutable revision)".to_string()
        ));

        let s = get_suggestions("The revision has conflicts");
        assert!(
            s.contains(&"Try running: jj resolve (to open the external merge tool)".to_string())
        );
    }
}
