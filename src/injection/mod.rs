use arboard::Clipboard;

// Copy text to clipboard
pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| format!("Failed to open clipboard: {e}"))?;
    clipboard
        .set_text(text.to_string())
        .map_err(|e| format!("Failed to write to clipboard: {e}"))?;
    println!("[WhisperShell] ✅ Transcript copied to clipboard ({} chars)", text.len());
    Ok(())
}
